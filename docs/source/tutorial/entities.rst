Entities
========

While :term:`blocks<block>` define the terrain of your world, entities are the
actors in the world: the objects that make things happen, or that your players
make thing happen to.

In fact, you’ve already seen one kind of entity: the :term:`player` is a
(special) kind of entity.

The flag entity
---------------

Unlike blocks, there are no pre-defined entities. Click on to the “Entities”
tab of the Hytopia Editor, and you’ll see nothing listed there: we’ll change
that in a second.

Our game needs only one :term:`type of entity<entity type>`: the flag. However,
there will be two instances of the flag :term:`entity` in the game: one for
each team. As we go along, we’ll see how one entity type can be used to fill
both roles.

Creating a new entity type using ``hy``:

.. code-block:: console

    $ hy entity_type flag

(As with block types, you’ll see the Flag automatically appear in the Hytopia
Editor.)

The new entity type is a directory containing information about the entity. The
resulting directory structure will look like:

.. code-block:: console

    $ tree -F CaptureTheFlag/entitytype/flag
    CaptureTheFlag/entitytype/flag/
    ├── behavior.ts
    ├── model.gltf
    └── property_types.json

The ``behavior.ts`` controls what the entity does and how it responds to the
world. We’ll see in a second what this practically means.

The ``model.gltf`` is the 3D model to use for the entity, in Graphics Library
Transmission Format (GLTF) format. The default, which ``hy`` has set us up
with, is a very uninspiring gray cube. Make a nicer flag in your favorite 3D
modelling program (for example, `Blockbench <https://www.blockbench.net/>`), or
you can use this one we prepared earlier:

**TODO: sample model goes here**

The ``property_types.json`` file defines what :term:`entity properties<entity
property>` each instance of this entity type is allowed to have. Properties let
instances of the same entity have different behaviour: for example, you might
use them to customise the texture displayed on the flag, how many hitpoints a
mob has, or the inventory of a chest.

Here, we’ll use a property to say which team owns the flag.

Open up ``property_types.json`` in your editor, and add the property:

.. code-block:: json

   [
     {
       "name": "owning_team",
       "options": ["red", "blue"],
       "default": "red",
     }
   ]

(Refer to **REFERENCE TO BE WRITTEN** for the full list of allowable property
types.)

Planting the flag
-----------------

Switch back to the Hytopia Editor: the flag is now ready to go.

Place a flag in the red base: switch to the Entities tab in the editor, click
on the flag, then click at an appropriate location in the red base to place
it.

The flag will be selected, and its properties will appear in the Properties
panel (bottom right of the editor). There’s only one custom property for this
entity (which we created above), but you’ll also see some generic system
properties common to all entities. We specified Red Team as the default owner
of this flag, so you don’t need to change anything here.

Place a second flag, in the blue base. This time, change the owning team
property to Blue Team.

Adding behavior
---------------

Before moving on to define the player’s behaviors (which will tie together the
flag, flag pedestal, and player), let’s add a simple, self-contained behavior
to the flag entity type.

The flag is an important entity, and it should draw attention to itself. The
flag should gently bob before it's picked up.

Open up ``behavior.ts`` in the Flag entity type. You’ll see that ``hy`` has
automatically generated boilerplate code for the most common events. Scroll
down to the ``onUpdate`` handler:

.. code-block:: typescript

    entityType.onUpdate( (entity, world) => {
        const bobLength = 3. * Hytopia.tick_rate_Hz;
        const yOffset = Math.sin(2 * Math.PI * world.tick / bobLength);
        entity.transform.y = entity.initialProperties.transform.y + yOffset - 2;
    } );

**PC NOTE: feel free to totally rewrite the code above, it could be anything**

**PC NOTE: when does gravity apply to entities? Perhaps we’d have to exempt
this one for this trick to work**

The onUpdate handler is called for every game :term:`tick`, that is, 60 times a
second. Here we simply have the flag’s vertical position bob every three
seconds.

Next up: the game
-----------------

Put the game in Playtest Mode, and head over to one of the bases to see the
flag behavior in action.

:doc:`Move to the next lesson, putting everything together </tutorial/capture-the-flag>`
