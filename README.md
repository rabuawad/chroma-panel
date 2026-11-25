# Chroma Panel

A native Rust/GPUI color-filter panel for Ubuntu. Choose Grayscale, Sepia or Invert, then use the intensity slider to blend the effect from 0% to 100%. The switch lets you compare the result with the original while keeping your chosen intensity.

## Run the panel

Run `cargo run --locked` in this directory. The pencil preview works independently of the GNOME extension. Use Tab or Shift+Tab to move between controls, Space or Enter to activate a switch or filter button, and arrow keys to adjust the slider by one percentage point. Home and End select the slider endpoints.

The build uses Rust 2024 and the pinned gpui-pre snapshot of Zed's GPUI. It was built on Ubuntu 26.04 with Rust 1.99 and tested on Wayland and X11. On a fresh machine, install Rust with Cargo, a C/C++ build toolchain, pkg-config, the Linux development libraries for fontconfig, Wayland, X11, XCB and xkbcommon, and a working Vulkan driver.

## Apply filters across the desktop

The included extension targets GNOME Shell 50 on Ubuntu 26.04. Check the shell version with `gnome-shell --version`.

1. Run `./install-extension.sh` from this directory. It installs the extension and compiles its settings schema under your user account.
2. Log out and back in so GNOME discovers the extension.
3. Run `gnome-extensions enable chroma-panel@local`.
4. Reopen the panel. The status should read **Desktop filter · GNOME**. Turn on **Color filters** to apply the effect.

GNOME stores the desktop settings and continues applying them after the panel closes. You can disable the filter from the switch or unload the extension with `gnome-extensions disable chroma-panel@local`. Preview-only settings last for the current run.

The panel communicates with the extension through the session D-Bus. GNOME applies the shader once to the desktop, including the panel's own preview. The pencil artwork is cropped from the supplied reference image.
