Blocks: defining terrain
========================

Hytopia is inspired by voxel-based games, meaning that the terrain of the world
is made from “blocks” that sit on a fixed grid. Each point in that grid can
have only one block on it.

Each :term:`world` in Hytopia is based on a 1,000×100×1,000 grid (1,000 blocks in the
east/west and north/south directions, and 100 blocks in the up/down direction).
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
dev CaptureTheCrab`` to re-open it).

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
``CaptureTheCrab`` directory.

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
 * wood
 * construct (an off-white block, like in *The Matrix*)

There's a limit of 255 different block types in a single world, so you’re free
to delete or repurpose any of the pre-defined block types if you’d prefer.

A new block type
----------------

We n
