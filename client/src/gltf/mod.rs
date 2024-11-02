use anyhow::anyhow;
use glam::{Mat4, Quat, UVec2, Vec2, Vec3, Vec4};
use gltf::{Animation, Glb, Mesh, Node};
use itertools::izip;

use crate::{console_log, transform::Transform};

#[derive(Debug, Default, Clone)]
pub struct GLTFModel {
    pub meshes: Vec<GLTFMesh>,
    pub skins: Vec<Skin>,
    pub nodes: Vec<GLTFNode>,
    pub root_node: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Joint {
    pub target_joint_node: usize,
    pub inverse_bind_matrix: Mat4,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Skin {
    pub joints: Vec<Joint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationLayer {
    pub name: String,
    pub index: usize,
    pub channels: Vec<AnimationChannel>,
    pub duration: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationChannel {
    pub time_values: Vec<f32>,
    pub output_values: Vec<glam::Vec4>,
    pub path: AnimationPath,
}

#[derive(Debug, Clone, PartialEq, Copy, Eq, Hash)]
pub enum AnimationPath {
    Position,
    Rotation,
    Scale,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct GLTFVertex {
    pub position: Vec4,
    pub normal: Vec4,
    pub joint: [u16; 4],
    pub weight: [f32; 4],
    pub uv: Vec2,
}

#[derive(Debug, Default, Clone)]
pub struct GLTFMesh {
    pub primitives: Vec<Primitive>,
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

impl Default for GLTFMaterial {
    fn default() -> Self {
        Self {
            base_colour_texture: Default::default(),
            base_colour_factor: Vec4::ONE,
            roughness_factor: 1.,
            metallic_factor: 1.,
            normal_texture: Default::default(),
            metallic_roughness_ao_texture: Default::default(),
            emissive_texture: Default::default(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct GLTFNode {
    pub children: Vec<usize>,
    pub transform: Transform,
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
pub struct Primitive {
    pub vertices: Vec<GLTFVertex>,
    pub indices: Vec<u32>,
    pub material: GLTFMaterial,
}

impl std::fmt::Debug for Primitive {
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

    let glb = Glb::from_slice(&file).unwrap();
    let root = gltf::json::Root::from_slice(&glb.json)?;
    let document = gltf::Document::from_json(root)?;
    let blob = glb.bin.ok_or_else(|| anyhow!("No binary found in glTF"))?;

    let root_node;
    if let Some(default_scene) = document.default_scene() {
        root_node = default_scene.nodes().next();
    } else if let Some(root) = document.nodes().next() {
        root_node = Some(root)
    } else {
        anyhow::bail!("No root node found in glTF");
    }

    for mesh in document.meshes() {
        asset.meshes.push(load_mesh(mesh, &blob)?);
    }
    for skin in document.skins() {
        asset.skins.push(load_skin(skin, &blob)?);
    }
    for node in document.nodes() {
        asset.nodes.push(load_node(node)?);
    }

    let mut animation_layers = Vec::new();
    for animation in document.animations() {
        animation_layers.push(load_animation(animation, &blob)?);
    }

    return Ok(asset);
}

fn load_node(node: Node<'_>) -> anyhow::Result<GLTFNode> {
    let transform = cvt(node.transform());
    let children = node.children().map(|n| n.index()).collect();
    Ok(GLTFNode {
        transform,
        children,
    })
}

fn load_animation(animation: Animation<'_>, blob: &[u8]) -> anyhow::Result<AnimationLayer> {
    let name = animation.name().unwrap_or_else(|| {
        console_log!(
            "Unable to load animation without name at index {}",
            animation.index()
        );
        "unknown"
    });

    console_log!("Loading animation {:?}", animation.name());
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

fn load_mesh(mesh: Mesh<'_>, blob: &[u8]) -> anyhow::Result<GLTFMesh> {
    console_log!("Loading primitives for {}", mesh.index());
    let mut parsed = GLTFMesh::default();
    for primitive in mesh.primitives() {
        let vertices = import_vertices(&primitive, &blob)?;
        let indices = import_indices(&primitive, &blob)?;
        let material = load_material(&primitive, &blob)?;

        let prim = Primitive {
            vertices,
            indices,
            material,
        };
        console_log!("Loaded primitive {:?}", prim);
        parsed.primitives.push(prim);
    }

    Ok(parsed)
}

fn load_skin(skin: gltf::Skin<'_>, blob: &[u8]) -> anyhow::Result<Skin> {
    console_log!("Loading skin index {}", skin.index());
    let inverse_bind_matrices = skin
        .reader(|_| Some(blob))
        .read_inverse_bind_matrices()
        .ok_or_else(|| {
            anyhow::anyhow!("Loading skins without inverse bind matrices is not supported")
        })?;

    let mut skin_component = Skin::default();
    for (joint, inverse_bind_matrix) in skin.joints().zip(inverse_bind_matrices) {
        skin_component.joints.push(Joint {
            target_joint_node: joint.index(),
            inverse_bind_matrix: Mat4::from_cols_array_2d(&inverse_bind_matrix),
        });
    }

    Ok(skin_component)
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

    let mut weights = Vec::new();
    let mut joints = Vec::new();

    if let Some(weight_reader) = reader.read_weights(0) {
        for weight in weight_reader.into_f32() {
            weights.push(weight);
        }
    } else {
        for _ in 0..position_reader.len() {
            weights.push(Default::default());
        }
    }

    if let Some(joint_reader) = reader.read_joints(0) {
        joints.extend(joint_reader.into_u16());
    } else {
        for _ in 0..position_reader.len() {
            joints.push(Default::default());
        }
    }

    let uv_reader = reader
        .read_tex_coords(0)
        .ok_or_else(|| anyhow!("Primitive has no UVs"))?
        .into_f32();
    let vertices = izip!(position_reader, normal_reader, weights, joints, uv_reader)
        .map(|(position, normal, weight, joint, uv)| GLTFVertex {
            position: Vec3::from(position).extend(1.),
            normal: Vec3::from(normal).extend(1.),
            weight,
            joint,
            uv: uv.into(),
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

fn load_material(primitive: &gltf::Primitive<'_>, blob: &[u8]) -> anyhow::Result<GLTFMaterial> {
    let material = primitive.material();
    let pbr = material.pbr_metallic_roughness();
    let base_colour_factor = pbr.base_color_factor().into();
    let roughness_factor = pbr.roughness_factor();
    let metallic_factor = pbr.metallic_factor();

    let base_colour_texture = load_texture(pbr.base_color_texture(), blob)
        .map_err(|e| console_log!("Unable to import base colour texture: {e}"))
        .ok();

    let normal_texture = load_texture(material.normal_texture(), blob)
        .map_err(|e| console_log!("Unable to import normal texture: {e}"))
        .ok();

    let metallic_roughness_ao_texture = load_texture(pbr.metallic_roughness_texture(), blob)
        .map_err(|e| console_log!("Unable to import metallic roughness AO texture: {e}"))
        .ok();

    let emissive_texture = load_texture(material.emissive_texture(), blob)
        .map_err(|e| console_log!("Unable to import emissive texture: {e}"))
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

fn load_texture<'a, T>(texture: Option<T>, blob: &[u8]) -> anyhow::Result<GLTFTexture>
where
    T: AsRef<gltf::Texture<'a>>,
{
    let texture = texture
        .as_ref()
        .ok_or_else(|| anyhow!("Texture does not exist"))?
        .as_ref();

    let view = match texture.source().source() {
        gltf::image::Source::View {
            view,
            mime_type: "image/png",
        } => Ok(view),
        gltf::image::Source::View { mime_type, .. } => {
            Err(anyhow!("Invalid mime_type {mime_type}"))
        }
        gltf::image::Source::Uri { .. } => Err(anyhow!("Importing images by URI is not supported")),
    }?;
    let start = view.offset();
    let end = view.offset() + view.length();

    let image_bytes = blob
        .get(start..end)
        .ok_or_else(|| anyhow!("Unable to read from blob with range {start}..{end}"))?;
    let image = image::load_from_memory(image_bytes)?.into_rgba8();

    Ok(GLTFTexture {
        dimensions: image.dimensions().into(),
        data: image.to_vec(),
    })
}
