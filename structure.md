# Structure

## Program:

The program is divided into several parts;

1. The compositor is responsible for arranging frames on screen, as well as the placement of individual displays
   relative to each-other. For optimisation reasons, the compositor is designed to only render the sections which
   strictly require it. This may occur through various events which are deemed to update the display. Among these are
   the following:
    * Cursor movement
    * Frame movement such as resizing, moving, or closing
    * Display movement
    * Explicit request to redraw (happens rarely, but every few seconds to keep the display fresh.)
2. The plugin manager allows Lua scripts to interact with the compositor or various other parts of the program. This
   allows user scripts complete control over the ongoings of the compositor. For instance, plugins can:
    * Control the placement and composition of frames
    * Generate, block or otherwise intercept (possibly modifying) events, such as mouse-movement or various frame events
    * Control the placement of displays
    * Generally alter the behaviour of the compositor
3. Event loop hosts the event queue where all events and requests pass through. The event loop is responsible for
   receiving and dispatching events to/from their required destinations. The event loop also provides the primary IPC
   mechanism by which clients communicate with the compositor through the use of Redox schemes. The scheme treats each
   file-descriptor as a frame, where the following events can take place:
    * `open` - A new frame is created
    * `read` - A single event is read from the frame's local event queue
    * `write` - A single request is written to the frame's local request queue
    * `close` - The frame is closed
    * `stat` - The frame's status is returned, giving information about its:
        * `id` - The frame's unique identifier (Not to be confused with the file-descriptor)
        * `title` - The frame's title
        * dimensions and dimension constraints
        * plugin-specific data such as assigned flags or memos.
    * `sync` - The frame is synchronised with the compositor, signaling to it that the client is still
      alive. This also refreshes the event and request queues, as well as **generating a redraw request**.
4. The display manager is responsible for managing display locations, and mapping coordinates between surface space and
   local space. This allows the cursor to be drawn directly on the display, rather than on the surface, where it must
   undergo a redraw to appear on screen, allowing the cursor to feel more responsive.
5. Config manager translates internal program state to/from a serialisable config format which is used to resume
   sessions after a crash or reboot. 

## Interaction

These pieces interact with one-another through the event system. 
