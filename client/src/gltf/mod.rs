use std::{collections::HashMap, path::Path, time::Duration};

use anyhow::{anyhow, Context};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, UVec2, Vec2, Vec3, Vec4};
use gltf::{Animation, Glb, Mesh, Node};
use image::buffer::ConvertBuffer;
use itertools::izip;

use crate::transform::Transform;

#[derive(Debug, Default, Clone)]
pub struct GLTFModel {
    pub meshes: Vec<GLTFMesh>,
    pub nodes: Vec<GLTFNode>,
    pub animations: Vec<AnimationLayer>,
    pub animation_state: AnimationState,
    pub root_node_idx: usize,
}

#[derive(Debug, Default, Clone)]
pub enum AnimationState {
    // Animation is enabled and gaining time every frame
    Playing {
        anim_index: usize,
    },
    // Animation enabled but is not gaining time.
    Paused,
    // Animation is fading from one to another
    Transitioning {
        from_index: Option<usize>,
        to_index: Option<usize>,
        // Duration of the transition
        duration: f32,
        // Current progress of the transition [0, 1]
        progress: f32,
    },
    // Animation is disabled
    #[default]
    Disabled,
}
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationLayer {
    pub name: String,
    pub index: usize,
    pub channels: Vec<AnimationChannel>,
    pub duration: f32,

    pub animation_time: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationChannel {
    pub time_values: Vec<f32>,
    pub output_values: Vec<glam::Vec4>,
    // Node index to target
    pub target_index: usize,
    pub path: AnimationPath,
}

#[derive(Debug, Clone, PartialEq, Copy, Eq, Hash)]
pub enum AnimationPath {
    Position,
    Rotation,
    Scale,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct GLTFVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Debug, Default, Clone)]
pub struct GLTFMesh {
    pub primitives: Vec<GLTFPrimitive>,
}

#[derive(Debug, Clone)]
pub struct GLTFMaterial {
    pub base_colour_texture: Option<GLTFTexture>,
    pub base_colour_factor: Vec4,
    pub roughness_factor: f32,
    pub metallic_factor: f32,
    pub normal_texture: Option<GLTFTexture>,
    pub metallic_roughness_ao_texture: Option<GLTFTexture>,
    pub emissive_texture: Option<GLTFTexture>,
}

#[derive(Debug, Default, Clone)]
pub struct GLTFNode {
    pub mesh: Option<usize>,
    pub children: Vec<usize>,
    pub base_transform: Transform,
    pub current_transform: Transform,
}

#[derive(Clone)]
pub struct GLTFTexture {
    /// x, y
    pub dimensions: UVec2,
    /// data is assumed to be R8G8B8A8
    pub data: Vec<u8>,
}

impl std::fmt::Debug for GLTFTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GLTFTexture")
            .field("dimensions", &self.dimensions)
            .field("data", &self.data.len())
            .finish()
    }
}

#[derive(Clone)]
pub struct GLTFPrimitive {
    pub vertices: Vec<GLTFVertex>,
    pub indices: Vec<u32>,
    pub material: GLTFMaterial,
}

impl std::fmt::Debug for GLTFPrimitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Primitive")
            .field("vertices", &self.vertices.len())
            .field("indices", &self.indices.len())
            .field("material", &self.material)
            .finish()
    }
}

pub fn load(file: &[u8]) -> anyhow::Result<GLTFModel> {
    let mut asset = GLTFModel::default();

    let gltf = gltf::Gltf::from_slice_without_validation(&file).unwrap();
    let document = gltf.document;
    let base_path: Option<&Path> = Some(Path::new(
        "We should not ever be reading a file or a non-data URI.",
    ));
    let buffers = gltf::import_buffers(&document, base_path, gltf.blob)
        .context("Could not find binary data")?;
    let textures =
        gltf::import_images(&document, base_path, &buffers).context("Failed to load textures")?;
    assert_eq!(buffers.len(), 1);

    let blob = &buffers[0];

    let root_node;
    if let Some(default_scene) = document.default_scene() {
        root_node = default_scene.nodes().next();
    } else if let Some(root) = document.nodes().next() {
        root_node = Some(root)
    } else {
        anyhow::bail!("No root node found in glTF");
    }
    asset.root_node_idx = root_node.context("No root node found in glTF")?.index();

    for mesh in document.meshes() {
        asset.meshes.push(load_mesh(mesh, &blob, &textures)?);
    }
    for node in document.nodes() {
        asset.nodes.push(load_node(node)?);
    }
    for animation in document.animations() {
        asset.animations.push(load_animation(animation, &blob)?);
    }

    return Ok(asset);
}

fn load_node(node: Node<'_>) -> anyhow::Result<GLTFNode> {
    let transform = cvt(node.transform());
    let children = node.children().map(|n| n.index()).collect();
    Ok(GLTFNode {
        mesh: node.mesh().map(|m| m.index()),
        current_transform: transform,
        base_transform: transform,
        children,
    })
}

fn load_animation(animation: Animation<'_>, blob: &[u8]) -> anyhow::Result<AnimationLayer> {
    let name = animation.name().unwrap_or_else(|| {
        tracing::info!(
            "Unable to load animation without name at index {}",
            animation.index(),
        );
        "unknown"
    });

    tracing::info!("Loading animation {:?}", animation.name());
    let mut channels = Vec::new();
    for channel in animation.channels() {
        // Try to get the path this channel is targeting
        let target = channel.target();

        // If we don't support this target, then ignore it
        let Some(path) = get_animation_path(target.property()) else {
            continue;
        };

        let reader = channel.reader(|_| Some(blob));
        let Some(inputs) = reader.read_inputs() else {
            anyhow::bail!("Unable to load animation without inputs");
        };
        let Some(channel_outputs) = reader.read_outputs() else {
            anyhow::bail!("Unable to load animation without outputs");
        };
        let inputs = inputs.into_iter().collect();
        let mut outputs = Vec::new();

        match channel_outputs {
            gltf::animation::util::ReadOutputs::Translations(translations) => {
                for t in translations {
                    outputs.push(Vec3::from(t).extend(1.))
                }
            }
            gltf::animation::util::ReadOutputs::Rotations(rotations) => {
                for r in rotations.into_f32() {
                    outputs.push(r.into())
                }
            }
            gltf::animation::util::ReadOutputs::Scales(scales) => {
                for s in scales {
                    outputs.push(Vec3::from(s).extend(1.))
                }
            }
            _ => continue,
        }

        let channel = AnimationChannel {
            time_values: inputs,
            output_values: outputs,
            target_index: target.node().index(),
            path,
        };

        channels.push(channel);
    }

    let Some(duration) = channels[0].time_values.iter().max_by(|a, b| a.total_cmp(b)) else {
        anyhow::bail!("No channels were found in this animation - this should NEVER happen");
    };
    let duration = *duration;

    Ok(AnimationLayer {
        index: animation.index(),
        name: name.into(),
        channels,
        duration,

        animation_time: 0.0,
    })
}

fn get_animation_path(property: gltf::animation::Property) -> Option<AnimationPath> {
    match property {
        gltf::animation::Property::Translation => Some(AnimationPath::Position),
        gltf::animation::Property::Rotation => Some(AnimationPath::Rotation),
        gltf::animation::Property::Scale => Some(AnimationPath::Scale),
        _ => None,
    }
}

fn load_mesh(
    mesh: Mesh<'_>,
    blob: &[u8],
    textures: &[gltf::image::Data],
) -> anyhow::Result<GLTFMesh> {
    tracing::info!("Loading primitives for {}", mesh.index());
    let mut parsed = GLTFMesh::default();
    for primitive in mesh.primitives() {
        let vertices = import_vertices(&primitive, &blob)?;
        let indices = import_indices(&primitive, &blob)?;
        let material = load_material(&primitive, textures)?;

        let prim = GLTFPrimitive {
            vertices,
            indices,
            material,
        };
        tracing::info!("Loaded primitive {:?}", prim);
        parsed.primitives.push(prim);
    }

    Ok(parsed)
}

fn import_vertices(
    primitive: &gltf::Primitive<'_>,
    blob: &[u8],
) -> anyhow::Result<Vec<GLTFVertex>> {
    let reader = primitive.reader(|_| Some(blob));
    let position_reader = reader
        .read_positions()
        .ok_or_else(|| anyhow!("Primitive has no positions"))?;
    let normal_reader = reader
        .read_normals()
        .ok_or_else(|| anyhow!("Primitive has no normals"))?;

    let uv_reader = reader
        .read_tex_coords(0)
        .ok_or_else(|| anyhow!("Primitive has no UVs"))?
        .into_f32();
    let vertices = izip!(position_reader, normal_reader, uv_reader)
        .map(|(position, normal, uv)| GLTFVertex {
            position,
            normal,
            uv,
        })
        .collect();
    Ok(vertices)
}

fn import_indices(primitive: &gltf::Primitive<'_>, blob: &[u8]) -> anyhow::Result<Vec<u32>> {
    let reader = primitive.reader(|_| Some(blob));
    let indices = reader
        .read_indices()
        .ok_or_else(|| anyhow!("Primitive has no indices"))?
        .into_u32()
        .collect();
    Ok(indices)
}

fn load_material(
    primitive: &gltf::Primitive<'_>,
    textures: &[gltf::image::Data],
) -> anyhow::Result<GLTFMaterial> {
    let material = primitive.material();
    let pbr = material.pbr_metallic_roughness();
    let base_colour_factor = pbr.base_color_factor().into();
    let roughness_factor = pbr.roughness_factor();
    let metallic_factor = pbr.metallic_factor();

    let base_colour_texture = load_texture(pbr.base_color_texture(), textures)
        .map_err(|e| anyhow!("Unable to import base colour texture: {e}"))
        .ok();

    let normal_texture = load_texture(material.normal_texture(), textures)
        .map_err(|e| tracing::info!("Unable to import normal texture: {e}"))
        .ok();

    let metallic_roughness_ao_texture = load_texture(pbr.metallic_roughness_texture(), textures)
        .map_err(|e| tracing::info!("Unable to import metallic roughness AO texture: {e}"))
        .ok();

    let emissive_texture = load_texture(material.emissive_texture(), textures)
        .map_err(|e| tracing::info!("Unable to import emissive texture: {e}"))
        .ok();

    Ok(GLTFMaterial {
        base_colour_texture,
        base_colour_factor,
        roughness_factor,
        metallic_factor,
        normal_texture,
        metallic_roughness_ao_texture,
        emissive_texture,
    })
}

fn cvt(transform: gltf::scene::Transform) -> Transform {
    let (position, rotation, _) = transform.decomposed();
    let rotation = Quat::from_array(rotation);
    Transform {
        position: position.into(),
        rotation,
        scale: glam::Vec3::ONE,
    }
}

fn load_texture<'a, T>(
    texture: Option<T>,
    textures: &[gltf::image::Data],
) -> anyhow::Result<GLTFTexture>
where
    T: AsRef<gltf::Texture<'a>>,
{
    let texture = texture
        .as_ref()
        .ok_or_else(|| anyhow!("Texture does not exist"))?
        .as_ref();

    let texture_data = &textures[texture.index()];

    // CRIME(cw): gltf import makes an image, then unpacks all that, then we put it back together for easy conversion.
    let rgba8_image = match texture_data.format {
        gltf::image::Format::R8G8B8A8 => image::RgbaImage::from_vec(
            texture_data.width,
            texture_data.height,
            texture_data.pixels.clone(),
        )
        .unwrap(),
        gltf::image::Format::R8G8B8 => image::RgbImage::from_vec(
            texture_data.width,
            texture_data.height,
            texture_data.pixels.clone(),
        )
        .unwrap()
        .convert(),
        gltf::image::Format::R8G8 => image::GrayAlphaImage::from_vec(
            texture_data.width,
            texture_data.height,
            texture_data.pixels.clone(),
        )
        .unwrap()
        .convert(),
        gltf::image::Format::R8 => image::GrayImage::from_vec(
            texture_data.width,
            texture_data.height,
            texture_data.pixels.clone(),
        )
        .unwrap()
        .convert(),
        _ => {
            return Err(anyhow!(
                "Unsupported texture format {:?}",
                texture_data.format
            ));
        }
    };

    Ok(GLTFTexture {
        dimensions: rgba8_image.dimensions().into(),
        data: rgba8_image.into_vec(),
    })
}

pub fn animate_model(model: &mut GLTFModel, time: Duration) {
    // For each node, store the animated value for each animation

    'state_change: loop {
        match model.animation_state {
            AnimationState::Playing { anim_index } => {
                let animation = &mut model.animations[anim_index];
                animation.animation_time +=
                    (animation.animation_time + time.as_secs_f32()) % animation.duration;

                for channel in &animation.channels {
                    let Some(value) = get_next_value_for_channel(channel, animation.animation_time)
                    else {
                        continue;
                    };

                    let node = &mut model.nodes[channel.target_index];

                    apply_value_to_node(node, channel.path, value);
                }
            }
            AnimationState::Paused => {}
            AnimationState::Transitioning {
                from_index,
                to_index,
                duration,
                ref mut progress,
            } => {
                *progress += time.as_secs_f32();

                if *progress >= duration {
                    if let Some(to_index) = to_index {
                        model.animation_state = AnimationState::Playing {
                            anim_index: to_index,
                        };
                    } else {
                        model.animation_state = AnimationState::Disabled;
                    }
                    break 'state_change;
                }

                let lerp_weight = *progress / duration;

                // Accumulate the sparse changes for the from animation and the to animation.
                let mut changes: HashMap<(usize, AnimationPath), (Option<Vec4>, Option<Vec4>)> =
                    HashMap::new();

                let from_animation = from_index.map(|i| &mut model.animations[i]);
                if let Some(from) = from_animation {
                    from.animation_time =
                        (from.animation_time + time.as_secs_f32()) % from.duration;

                    for channel in &from.channels {
                        let Some(value) = get_next_value_for_channel(channel, from.animation_time)
                        else {
                            continue;
                        };

                        let change = changes
                            .entry((channel.target_index, channel.path))
                            .or_default();
                        change.0 = Some(value);
                    }
                }

                let to_animation = to_index.map(|i| &mut model.animations[i]);
                if let Some(to) = to_animation {
                    to.animation_time = (to.animation_time + time.as_secs_f32()) % to.duration;

                    for channel in &to.channels {
                        let Some(value) = get_next_value_for_channel(channel, 0.0) else {
                            continue;
                        };

                        let change = changes
                            .entry((channel.target_index, channel.path))
                            .or_default();
                        change.1 = Some(value);
                    }
                }

                for ((node_index, path), (from_state, to_state)) in changes {
                    let from_state = from_state
                        .unwrap_or_else(|| get_default_pose_for_channel(model, node_index, path));

                    let to_state = to_state
                        .unwrap_or_else(|| get_default_pose_for_channel(model, node_index, path));

                    let final_state = lerp_anim(path, from_state, to_state, lerp_weight);

                    apply_value_to_node(&mut model.nodes[node_index], path, final_state);
                }
            }
            AnimationState::Disabled => {
                for node in &mut model.nodes {
                    node.current_transform = node.base_transform;
                }
            }
        }
        break;
    }
}

fn lerp_anim(path: AnimationPath, from: Vec4, to: Vec4, factor: f32) -> Vec4 {
    match path {
        AnimationPath::Rotation => {
            let from = Quat::from_vec4(from);
            let to = Quat::from_vec4(to);
            from.slerp(to, factor).to_array().into()
        }
        _ => from.lerp(to, factor),
    }
}

fn apply_value_to_node(node: &mut GLTFNode, path: AnimationPath, value: Vec4) {
    match path {
        AnimationPath::Position => {
            node.current_transform.position = value.truncate();
        }
        AnimationPath::Rotation => {
            node.current_transform.rotation = Quat::from_vec4(value);
        }
        AnimationPath::Scale => {
            node.current_transform.scale = value.truncate();
        }
    }
}

fn get_default_pose_for_channel(model: &GLTFModel, node_idx: usize, path: AnimationPath) -> Vec4 {
    let node = &model.nodes[node_idx];
    match path {
        AnimationPath::Position => node.base_transform.position.extend(1.),
        AnimationPath::Rotation => node.base_transform.rotation.to_array().into(),
        AnimationPath::Scale => node.base_transform.scale.extend(1.),
    }
}

/// Get the next value for an animation channel.
///
/// Per the glTF spec we iterate through the inputs (a list of timestamps) and try to find
/// a "lower" and "higher" timestamp than `elapsed`. When we find these timestamps, we record
/// their indices, fetch the corresponding outputs (a list of Vec4s representing the required
/// transform) and lerp between them based on `lerp_s`: a scale that tells us how far "between"
/// elapsed is from "low" and "high".
///
/// This implementation is loosely based on the glTF tutorial:
/// https://github.com/KhronosGroup/glTF-Tutorials/blob/main/gltfTutorial/gltfTutorial_007_Animations.md
fn get_next_value_for_channel(channel: &AnimationChannel, current_time: f32) -> Option<glam::Vec4> {
    let mut previous_time = 0.;
    let mut next_time = f32::MAX;

    let mut previous_index = None;
    let mut next_index = None;

    let time_values = &channel.time_values;
    let output_values = &channel.output_values;

    // Fast path: if there is only one value, then that's what we have to return
    if output_values.len() == 1 {
        return output_values.first().copied();
    }

    for (index, time) in time_values.iter().enumerate() {
        let time = *time;

        // previous_time is the largest element from the times accessor that is smaller than current_time
        if time > previous_time && time < current_time {
            previous_time = time;
            previous_index = Some(index);
        }

        // next_time is the smallest element from the times accessor that is larger than current_time
        if time < next_time && time > current_time {
            next_time = time;
            next_index = Some(index);
        }
    }

    // Get the output values from the indices we found
    let previous_output = output_values[previous_index?];
    let next_output = output_values[next_index?];

    // Compute the interpolation value. This is a value between 0.0 and 1.0 that describes the relative
    // position of the current_time, between the previous_time and the next_time:
    let interpolation_value = (current_time - previous_time) / (next_time - previous_time);

    let next_value = match channel.path {
        AnimationPath::Rotation => Quat::from_array(previous_output.to_array())
            .slerp(
                Quat::from_array(next_output.to_array()),
                interpolation_value,
            )
            .to_array()
            .into(),
        _ => previous_output.lerp(next_output, interpolation_value),
    };

    Some(next_value)
}
