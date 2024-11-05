Getting started
===============

To demonstrate all the basic features of Hytopia, we’re going to build a game
of Capture The Flag (you can capture a Flag, or even something else, if you’d
prefer). The idea is simple: you, or another member of your team, need to grab
the other team’s Flag and get it back to your base, before the other team does
the same to you.

You’ll see how Hytopia lets you combine simple primitives to quickly build a
deceptively-complex game. The game you build will automatically be
multiplayer, have fast networking, handle a variety of devices and controllers,
and have robust error recovery and reporting. With a few clicks you’ll be able
to share the game you’ve made with your friends, put it on Hytopia’s
Marketplace for others to play, or use it as the base of your own creation.

You can follow along this tutorial at your own pace, and you should feel free
to add your own twists and turns to the basic game we lay out.

When you’re asked to type in a command during this tutorial, it’ll be shown
like this:

.. code-block:: console

    $ echo hi
    hi

You type: ``echo hi`` and you’ll see ``hi`` as the output.

 .. topic:: Challenge

 **Challenge**: Boxes like this will contain challenges for things you might
 want to do during this tutorial, to add a bit more to your first game.

How a Hytopia game is made
--------------------------

A Hytopia :term:`world` is defined by a directory containing Typescript (or
Javascript) scripts, JSON files, and other assets (such as models). Hytopia
comes with a tool, called ``hy``, to manage these directories: through this
tutorial, you’ll learn what each file does.

You’ll need just three tools to build your first world and game:

 * A web browser
 * A development environment or text editor configured to write Typescript or
   Javascript
 * The ``hy`` command line interface (CLI)

If you’re reading this documentation, you probably already have a web browser:
but if not, both `Chrome <https://www.google.com/chrome/>` and `Firefox
<https://getfirefox.com/>` work great.

You can use any integrated development environment (IDE) for editing Hytopia’s
Typescript files. We recommend **XXXXX**.

The ``hy`` CLI is part of Hytopia, and runs on Windows, Mac and Linux. If
you’ve used web toolkits like React, you’ll find interacting with ``hy`` feels
very similar to interacting with tools like ``create-react-app`` — and if you
haven’t, don’t worry: this tutorial will walk you through everything you need
to know.

Install ``hy`` by:

**XXX INSTALL INSTRUCTIONS HERE**

Your first world
----------------

It’s time to create a :term:`world`. In Hytopia, a “world” is the virtual
environment that any Hytopia game takes place in. (Other similar platforms
might call them “scenes”, “places” or “levels”.) As we’ll see, a world has
:term:`blocks<block>`, :term:`entities<entity>`, scripting to tie everything
together, and more. We’ll learn about these as the tutorial goes on, but in
short: everything you can do in Hytopia takes place in a world. As a developer,
your task is to come up with an engaging world that players enjoy and want to
spend time in.

Creating a world is simple:

.. code-block:: console

    $ hy create CaptureTheFlag


A few things have happened: First, ``hy`` has created a directory (helpfully
called ``CaptureTheFlag``). Second, your web browser has popped up showing you
the :term:`Hytopia Editor` environment, as well as your new (and
currently-empty) world. Third, ``hy`` has started the Hytopia Development
Server (you’ll be able to see output from the server in your terminal window).

Two commands you’ll find useful:

 * If you close the browser tab containing the :term:`Hytopia Editor`, you can
   always reload it with: ``hy dev CaptureTheFlag``
 * If you close the Hytopia Development Server (for example by pressing
   :kbd:`Ctrl+C`), you can restart it with: ``hy run CaptureTheFlag``

Although there’s not much in it, your world is already playable! Press the Play
icon in the editor **TODO SCREENSHOT HERE** and your Hytopia player will spawn
in the CaptureTheFlag world. This is Playtest Mode. The standard Hytopia
:kbd:`W`, :kbd:`S`, :kbd:`A`, :kbd:`D` keybinding will work automatically, and
you can run around.

Congratulations! You’ve made your first Hytopia world.

When you’ve had a look around, press the Stop icon **TODO SCREENSHOT HERE** to
return to Edit Mode.

What’s in the box?
------------------

Above, we mentioned that ``hy`` has created a directory containing the files
defining your world. Before we move on to the next stage of the tutorial, it’s
worth taking a brief look at the files that have been created for you.

Open up a terminal and list the ``CaptureTheFlag`` directory:

.. code-block:: console

    $ tree -F CaptureTheFlag
    CaptureTheFlag/
    ├── blocktypes/
    │   └── asphalt/
    │      ├── top.png
    │      ├── behavior.ts
    │      └── properties.json
    │   └── dirt/
     # ...........
    ├── entities.json
    ├── entitytypes/
    ├── grid.dat
    ├── metadata.json
    ├── player.ts
    ├── skybox/
    │   └── 0.png
    │   └── 1.png
    │   └── 2.png
    │   └── 3.png
    │   └── 4.png
    │   └── 5.png
    ├── world.json
    └── world.ts

We’ll meet these files again as we go through the tutorial, but here’s a brief
description of each, so you know what to expect.

The ``.json`` files (and ``grid.dat``) are edited by the Hytopia Editor. The
``.ts`` files, and the skybox, can be edited by you in an editor of your
choice.

``blocktypes``
  This directory defines the different :term:`block types<block type>` that are
  used in the world. This directory comes pre-populated with some basic,
  broadly-applicable block types. You’ll see how to use the prebuilt block
  types, and create your own new block type, in <tutorial/blocks>.

``entities.json``
  This file (which you don’t need to edit by hand) lists all the
  :term:`entities<entity>` in your world. It’s edited by the Hytopia Editor.

``blocktypes``
  This empty directory will contain definitions for the different :term:`entity
  types<entity type>` that will be used in the world. You’ll create some entity
  types in <tutorial/entities>.

``grid.dat``
  This file contains all the blocks in the world, stored in an efficient
  format. Again, it’s edited by the Hytopia Editor.

``metadata.json``
  This file contains some metadata describing your world, for example, the name
  of the world, and its author.

``player.ts``
  This file contains Typescript code that allows the player to react to event
  happening in the world. The template ``hy`` creates has some default
  behaviours, as well as boilerplate prebuilt code for the most common player
  events.

``skybox``
  This directory contains the six imagines that make up what the sky looks like
  in your world. As you saw when you played the game in the editor, the default
  sky is the color of television, tuned to a dead channel.

``world.json``
  This file defines some static properties of the world **TODO -- MAYBE NOT
  NEEDED**

``world.ts``
  This file contains Typescript code that controls how the world reacts to time
  passing. The template ``hy`` creates has boilerplate prebuilt code for the
  most common world events.

 .. topic:: Challenge

 **Challenge**: Edit ``metadata.json`` to set the name, author and description
 of your game.


Next up: blocks
---------------

Next you’ll learn how blocks define the terrain of the world.

:doc:`Move to the next lesson, on blocks <tutorial/blocks>`
