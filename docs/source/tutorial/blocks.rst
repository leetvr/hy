Blocks: defining terrain
========================

Hytopia is inspired by voxel-based games, meaning that the terrain of the world
is made from “blocks” that sit on a fixed grid. Each point in that grid can
have only one block on it.

Each :term:`world` in Hytopia is based on a 256×64×264 grid (256 blocks in the
east/west and north/south directions, and 64 blocks in the up/down direction).
These are referred to as the :math:`x`, :math:`y` and :math:`z` directions.
(Hytopia uses a right-handed co-ordinate system.) Anything outside this grid is
inaccessible to all players: they won’t be able
to get through the magic forcefield surrounding your world.

You’ll make the terrain of your game by placing blocks. Any area of the grid
that doesn’t have a block placed is treated as air, which
:term:`players<player>` can freely walk through.

Thinking back to when you played your freshly-created world, you might wonder
why you didn’t simply fall through the floor. The answer is that `hy create`
automatically places a plane of concrete blocks at :math:`y = 15` for you, so
you had somewhere to run around.

The Hytopia Editor
------------------

Switch to the :term:`Hytopia Editor` in your browser (you can always run ``hy
dev CaptureTheFlag`` to re-open it).

You can navigate the world in the Hytopia Editor in the following ways:

 * You can translate horizontally around the world with the :kbd:`W` :kbd:`W`,
   :kbd:`S`, :kbd:`A`, :kbd:`D` keys
 * You can translate up and down with :kbd:`R` and :kbd:`F`
 * You can pitch with :kbd:`T` and :kbd:`G`
 * You can yaw (pan) with :kbd:`Q` and :kbd:`E`
 * You can zoom in and out by scrolling the mousewheel
 * If it ever becomes all too hard, :kbd:`Z` will reset your view

**TODO: All keybinds above are literally just from me looking at my keyboard
and picking keys**

(If you find using those keys moves your character around, you’re still in
Playtest Mode. Press the Stop icon **TODO ICON HERE** to return to Edit Mode,
where your character is not visible and you can move freely.)

Practice moving around the editor for a bit, and when you’ve got the hang of
it, move on.

Pre-defined blocks
------------------

You’ll also see that ``hy create`` has created a series of pre-defined block
types: these are the directories under the ``blocktypes`` directory in your
``CaptureTheFlag`` directory.

These pre-defined block types are broadly useful, and we’ll place some now.

You can place blocks by clicking on the block type you want in the Block
Palette (left-hand side of the screen), and then clicking on a place in the
editor screen. Your mouse cursor will automatically snap so that a new block is
placed on top of the one beneath it.

You can delete a block by holding down :kbd:`Shift` while clicking.

You’ll probably find it tedious to click on each block individually to place
it.  You can click-and-drag to quickly add a lot of blocks of the same type:
note that when in click-and-drag mode, you’re locked to the vertical
(:math:`y`) position of the first block you place.

Make some attractive scenery for the two teams to compete. It’s worth having
some variation in elevation, block types, and theme.

 .. topic:: Challenge

 **Challenge**: Build a small maze at the entrance to each team’s base, with a
 couple of way through it. Use different block types to make the maze more
 interesting.

Pre-defined block types
.......................

The pre-defined block types made by ``hy create`` are:

 * air (special block type for grid spaces without a block)
 * asphalt
 * brick
 * concrete
 * dirt
 * glass (semi-transparent)
 * grass (grass on up-side only, otherwise, dirt)
 * metal plating
 * painted line, white (useful for roads)
 * painted line, yellow (useful for taxiways)
 * smoke (obscures vision somewhat, but players can walk through it)
 * stone
 * stone steps (non-standard block shape)
 * wood
 * construct (an off-white block, like in *The Matrix*)

There's a limit of 255 different block types in a single world, so you’re free
to delete or repurpose any of the pre-defined block types if you’d prefer.

A new block type
----------------

The pre-defined block types are great for terrain and some obstacles. However,
you might want to do something a bit different. And, while we’re here, we will
need to define a special block: the pedestal on which the player will put the
flag.

Creating a new block type is simple:

.. code-block:: console

    $ hy block_type flag_pedestal

(You can take a look in the Hytopia Editor and see that the new block type has
appeared automatically.)

This creates a new :term:`block type`, called ``flag_pedestal``. Most
practically, it creates a new directory like:

.. code-block:: console

    $ tree -F CaptureTheFlag/blocktype/flag_pedestal
    CaptureTheFlag/blocktype/flag_pedestal/
    ├── top.png
    └── properties.json

There are two files created for you.

The JSON file file ``properties.json`` controls whether or not the block can be
walked through. The flag pedestal isn’t such a block, so no changes are needed.

The file ``top.png`` is the texture shown on the block's faces. The following
names are recognised:

 * ``top.png`` -- topmost face
 * ``bottom.png`` -- bottommost face
 * ``right.png`` -- rightmost face (i.e., face pointing in positive :math:`x`
   direction)
 * ``left.png`` -- leftmost face (i.e., face pointing in negative :math:`x`
   direction)
 * ``forwards.png`` -- face pointing forwards (i.e., face pointing in positive
   :math:`z` direction)
 * ``backwards.png`` -- face pointing backwards (i.e., face pointing in
   negative :math:`z` direction)

If a texture file is missing, Hytopia will automatically use the texture for
the face above it in the list. This is why ``hy`` has only created a
``top.png``: the same texture will be automatically used on all six faces of
the cube.

The flag pedestal will look best if it’s sitting on top of the blocks
surrounding it. For that reason it would make sense to have a different face on
top compared to the sides.

 * Create a ``top.png`` that looks like a flag pedestal
 * Create a ``bottom.png`` that looks like a metal support. Note that this
   texture will automatically be used for the other four faces

The ``grass`` predefined block type is an example of this sort of block.

If you don’t have a texture you’d like to use, consider the following:

**TODO: SAMPLE TEXTURES**

 .. topic:: Challenge

 **Challenge**: In capture the flag, you want it to be quick to leave your base
 but somewhat more difficult to get in. Define a new ``wooden_steps`` block
 type with a stairs shape, and use it in your bases. (See stone steps for
 inspiration.)

Next up: entities
-----------------

You’re ready to move on to entities... but before you do that, click the Play
button and walk through your world!

None of the game behaviors are in place yet, but you can walk through your two
bases and work out what you might need to tweak.

When you’re ready, click the Stop button to move back to Edit mode, and:

:doc:`Move to the next lesson, on entities </tutorial/entities>`
