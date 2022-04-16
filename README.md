# Hutch - embedded compositor

**NOTE**: _Project is in early stage of development which means that you should not use it yet._

Simple, tiny and embeddable Compositor which can be ran inside either X11 or Wayland session.

### Development
Hutch is derived from "Smallvil", minimal (unreleased) example provided by [Smithay](https://github.com/Smithay/smithay) developers.  

Compositor itself is meant to be simple, tiny and embeddable inside your current session, either as X11 window or Wayland toplevel surface.  
There are no plans on implementing native udev backend for sanity purposes.

Future plans:
  - High priority
    - [ ] Automatic socket bind instead of hardcoded `wayland-3`.
    - [ ] Every toplevel should start maximized.
    - [ ] Add stdin argument to easly run apps.
    - [ ] `zwp-linux-dmabuf-v1` for running Vulkan programs.
    - [ ] XWayland implementation for running X11 programs.
    - [ ] Single function library for integrating Hutch with Game Engines.

  - Lower priority
    - [ ] Clipboard implementation.
    - [ ] `wlr-layer-shell-unstable-v1` implementation for [smithay-egui](https://github.com/Smithay/smithay-egui) to be used as debug menu.

### Contributing
Feel free to contribute bugfixes and implementations of new features and protocols listed above.  
You only have to follow formatting style from all `.rs` files listed in this repository (don't use rustfmt).

### License
Hutch is licensed under Mozilla Public License v2.0, see [LICENSE](LICENSE) for more details.
