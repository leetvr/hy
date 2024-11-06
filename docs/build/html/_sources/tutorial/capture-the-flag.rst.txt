Coding Capture the Flag
=======================

**PWC NOTE: I did all of this code freehand, no guarantee it makes sense/is
syntactically valid, and I'm not suggesting we actually use the API I’ve
defined — rather I tried to think of all the code that is necessary, and put it
in there; any other API design design will need to solve at least these
problems**

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

Most event handlers receive arguments in the following way:

 * the object that is handling the event: the entity, or player, affected by or
   causing the event
 * if relevant, the subject of the event (for example, what the player collided
   with)
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

Open up ``world.ts`` and locate the ``onSlashCommand`` function:

.. code-block:: typescript

    world.onSlashCommand( (player: Player, command: string, args: string[], world: World) => {
        if(command === "start") {
            // handle '/start'
            if(world.state.isRunning) {
                world.messagePlayer(player, "Game is already running, '/start' ignored");
            } else {
                world.state.isRunning = true;
                world.state.score = { "red": 0, "blue": 0 };
                world.state.checkForWin = (world: World) => {}; // We'll implement this later
                resetFlag(world.entityById('red-flag'));
                resetFlag(world.entityById('blue-flag'));
            }
        } else if(command === "end")
            if(world.state.isRunning) {
                endGame(world);
            } else {
                world.messagePlayer(player, "Game isn't running, '/end' ignored");
            }
        }
    } );

    function endGame(world: World) {
        world.state.isRunning = false;
        resetFlag(world.entityById('red-flag'));
        resetFlag(world.entityById('blue-flag'));
    }

    function resetFlag(flag: Flag) {
        flag.velocity = Vec3(0., 0., 0.);
        flag.transform = flag.initialProperties.transform;
        if(flag.state.carriedBy) {
            flag.state.carriedBy.state.carriedFlag = undefined;
            flag.state.carriedBy = undefined;
        }
    }

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

The default behavior is to spawn the player in the world just above the origin,
at :math:`(x, y, z) = (0, 20, 0)`. This is why your player spawned in Playtest
Mode.

We want to replace this behavior with something appropriate for this game.
We’ll assign each player to the red or blue team, depending on which team is
weaker, and spawn them in their team's base.

Replace the ``onRequestSpawn`` event handler as follows:

.. code-block:: typescript

    player.onRequestSpawn( (player: Player, world: World) => {
        // Check which team has fewer players
        const bluePlayers = countPlayers(world, "blue");
        const redPlayers = countPlayers(world, "red");

        if(redPlayers > bluePlayers) {
            player.state.team = "blue";
        } else {
            player.state.team = "red";
        }
        world.messagePlayer(player, "You are on the " + player.state.team.toUpperCase() + " team");

        // Spawn at a random point within x = ±4, z = ±4 of the base
        let spawnPoint: BlockPos = findBase(world, player.state.team).randomise(4, 0, 4);
        world.spawnPlayer(player, spawnPoint);
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
        // TODO: left as exercise to reader
    }

If you press the Play button now to switch to Playtest Mode, you’ll see you
spawn at the red base, and get a notice that you’re on the red team. The start
location is randomised slightly: you can restart the game a few times to see
this in action.

Picking up the flag
-------------------

When the player touches the flag, they should pick it up. We’d also like to
notify everyone in the game, so they know who to block.

The most useful event here is ``onCollideWithEntity`` in ``player.ts``:

.. code-block:: typescript

    player.onCollideWithEntity( (player: Player, entity: Entity, world: World) => {
        if(entity.entityType.name !== "flag") {
            // For now this test isn't strictly needed, but it's good practice
            // to include. As you develop more complex games, often
            // `onCollideWithEntity` and similar functions will call an
            // entity-specific function based on the type of the touched entity
            return;
        }

        // TODO -- there's a way to do this in typescript but I forget what it
        // is
        let flag: EntityType::Flag = entity;

        if(player.state.team === flag.getProperty('owning_team')) {
            if(flag.state.carriedBy === false) {
                // Can't pick up your own flag
                return;
            } else {
                // Challenge: Alter this so you can intercept an opposing
                // player and recover the flag
                return;
            }
        }

        if(player.state.carriedFlag) {
            return;
        }

        // Pick up flag
        player.state.carriedFlag = flag;
        flag.state.carriedBy = player;
        world.messageAll(player.name + " has the flag!");

    });

This sets up the metadata to keep track of who's carrying the flag. The other
thing we'll need to do is make the flag move with the player, so it’s being
properly carried.

Edit the entity's ``flag/behavior.ts`` file to change the ``onUpdate`` event
handler as follows:

.. code-block:: typescript

    entityType.onUpdate( (entity, world) => {
        if(entity.carriedBy) {
            // Follow my carrier
            let player: Player = entity.carriedBy;
            entity.transform = player.transform - 0.3*player.velocity;
        } else {
            // Bobbing behavior
            const bobLength = 3. * Hytopia.tick_rate_Hz;
            const yOffset = Math.sin(2 * Math.PI * world.tick / bobLength);
            entity.transform.y = entity.initialProperties.transform.y + yOffset - 2;
        }
    } );

Play the game again. You can pick up the flag (remember: you can't pick up the
flag in your base, you’ll have to go to the blue team base!). You might want to
tweak the calculation of ``entity.transform`` above to make the carrying action
look more natural.

Winning the flag
----------------

You get a point when you bring the flag back to the pedestal in your base.

You’ll recall we implemented the flag pedestal as a block, so we want to edit
the behavior of the player when we touch that block. In ``player.ts`` find the
relevant event handler:

.. code-block:: typescript

    player.onCollideWithBlock( (player: Player, block: Block, world: World) => {
        if(block.blockType !== "flag_pedestal") {
            return;
        }

        if(!player.state.carriedFlag) {
            return;
        }

        // TODO: figure out how to tell if we're in the red base or the blue
        // base
        if(theBaseImIn !== player.state.team) {
            return;
        }

        // Capture the flag!
        let flag: Flag = player.state.carriedFlag;

        world.state.score[player.state.team] += 1;
        world.state.checkForWin(world);
        world.messageAll(player.name + " captured the flag! One point to " + player.state.team.toUpperCase() + " team!");
        world.messageAll("Scores: RED " + world.state.score["red"] + " vs " + world.state.score["blue"] + " BLUE");
        resetFlag(flag);
    });

World behavior
--------------

The last thing we need to for our basic Capture The Flag is a scoring system.
At the moment scores are tracked, in ``world.state.scores``, and we want to end
the game if either team makes it to five captures.

Cleverly, we set aside a ``checkForWin`` function in the ``onSlashCommand``
event handler in ``world.ts``. We can now provide an implementation:

.. code-block:: typescript

   // ...
        world.state.checkForWin = (world: World) => {
            let winner: string? = undefined;
            if(world.state.score["red"] >= 5) {
                winner = "RED";
            } else if(world.state.score["blue"] >= 5) {
                winner = "BLUE";
            }
            if(winner) {
                world.globalMessage(winner + " team wins the game!");
                endGame(world);
            }
        };
   // ...


Play!
-----

You now have a basic, but fully playable, game of capture the flag. Have a play
around, using the editor’s Playtest mode, check you’re happy with everything,
and maybe tweak the logic a little bit.

Next up: you can either:

 * :doc:`add projectiles to your game </tutorial/projectiles>`
 * :doc:`set up your game so it can be played by others </tutorial/multiplayer>`
