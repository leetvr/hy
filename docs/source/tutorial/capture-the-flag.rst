Coding Capture the Flag
=======================

**PWC NOTE: I did all of this code freehand, no guarantee it makes sense/is
syntactically valid**

We’re now ready to put all the pieces together and turn this world into a game.

In this tutorial we’ll build the following functionality:

 * use the ``/start`` and ``/end`` commands to start the game
 * when a player joins, assign them a team, and spawn them near that team's base
 * when a player touches the opposing team's flag, pick it up
 * when a player with a flag touches their team's flag pedestal, give them a
   point
 * when a team reaches five points, they win and the game ends until restarted

Conventions and managing state
------------------------------

**TODO: PWC below I basically design "global variables yolo"**

Hytopia is intentionally flexible, to support a wide variety of games,
including ones we didn’t imagine when designing the platform. Despite this,
there are some commonly-used conventions we encourage you to adopt.

Hytopia uses an events-based model: games are primarily implemented by
responding to events, whether they be caused by the game itself (a player
touching something) or the passage of time (the regular ``onUpdate`` tick).

Most event handlers receive two arguments:

 * the object that is handling the event: the entity, or player, affected by or
   causing the event
 * the world itself, with access to all entities and global properties

Some event handlers have fewer or more arguments, but generally follow this
pattern.

All Hytopia entities, and the world itself, have a ``state`` object which you
can use for storing state (scores, story progression, etc) for your game.

Responding to ``/start`` and ``/end``
-------------------------------------

Although your game can intercept and respond to all chat messages, it's a
Hytopia convention to use the slash-commands ``/start`` and ``/end`` to control
whether game logic runs, and allow players to gather at a neutral start/end
period between game rounds.

The world provides special event handlers to make handling these commands
easy.

Open up ``World.ts`` and locate the ``onSlashCommand`` function:

.. code-block:: typescript

    world.onSlashCommand( (player: Player, command: string, args: string[], world: World) => {
        if(command === "start") {
            // handle '/start'
            if(world.state.isRunning) {
                world.messagePlayer(player, "Game is already running, '/start' ignored");
            } else {
                world.state.isRunning = true;
                world.state.score = { "red": 0, "blue": 0 };
                world.state
            }
        } else if(command === "end")
            if(world.state.isRunning) {
                world.state.isRunning = false;
            } else {
                world.messagePlayer(player, "Game isn't running, '/end' ignored");
            }
        }
    } );

Here we use an ``isRunning`` boolean in the world's state to control if the
game is running.

Player spawning
---------------

All player behavior, including spawning, is captured in ``player.ts``. If you
search for this, you’ll see that ``hy``’s template includes a default spawn
behavior:

.. code-block:: typescript

    player.onRequestSpawn( (player: Player, world: World) => {
        world.spawnPlayer(player, BlockPos(0, 20, 0));
    } );

The default behavior is to spawn the player in the world just above the origin
(:math:`(x, y, z) = (0, 20, 0)`). This is why your player spawned in Playtest
Mode.

We want to replace this behavior with something appropriate for this game.
We’ll assign each player to the red or blue team, depending on which team is
weaker, and spawn them in their team's base.

Replace the ``onRequestSpawn`` event handler as follows:

.. code-block:: typescript

    player.onRequestSpawn( (player: Player, world: World) => {
        const bluePlayers = countPlayers(world, "blue");
        const redPlayers = countPlayers(world, "red");

        if(redPlayers > bluePlayers) {
            player.team = "blue";
        } else {
            player.team = "red";
        }

        // Spawn at a random point within x = ±4, z = ±4 of the base
        let spawnPoint: BlockPos = findBase(world, player.team).randomise(4, 0, 4);
    } );

    function countPlayers(world: World, team: string): number {
        let result = 0;
        world.players.forEach( (p) => {
            if(p.state.team === team) {
                ++result;
            }
        });
        return result;
    }

    function findBase(world: World, team: string): BlockPos {
        // left as exercise to reader
    }

