# Chroma Panel

A native Rust/GPUI color-filter panel for Ubuntu. Choose Grayscale, Sepia, Invert or Tint, then use the intensity slider to blend the effect from 0% to 100%. The switch lets you compare the result with the original while keeping your chosen intensity.

## Run the panel

Run `cargo run --locked` in this directory. The color field previews the filter when the GNOME extension is unavailable. Use Tab or Shift+Tab to move between controls, Space or Enter to activate a switch or filter button, and arrow keys to move intensity to the next 10% mark. Clicking or dragging snaps to the nearest mark. Home and End select the slider endpoints.

The build uses Rust 2024 and the pinned gpui-pre snapshot of Zed's GPUI. It was built on Ubuntu 26.04 with Rust 1.99 and tested on Wayland and X11. On a fresh machine, install Rust with Cargo, a C/C++ build toolchain, pkg-config, the Linux development libraries for fontconfig, Wayland, X11, XCB and xkbcommon, and a working Vulkan driver.

## Choose a color

Turn on **Color filters** to tint the desktop as you pick a color. Picking a color selects **Tint** automatically. Intensity controls how strongly the tint affects the desktop.

Drag inside the color field to set saturation and brightness. The vertical strip selects hue. With Tint selected, open **Desktop preview** above the color field to see the chosen color and intensity on a mock Ubuntu desktop, even with desktop filtering switched off. The **Sliders** tab offers RGB controls, and **Palette** shows tints and shades of the current hue. Click the hex value to copy it.

Click a preset or recent swatch to use its color. The **+** beside Presets adds the current color. Click an empty Favorites slot to fill it, or right-click a filled slot to clear it. **Done** saves your colors and closes the window. **Cancel** discards color-library edits and restores the desktop filter settings from when the window opened.

## Apply filters across the desktop

The included extension targets GNOME Shell 50 on Ubuntu 26.04. Check the shell version with `gnome-shell --version`.

1. Run `./install-extension.sh` from this directory. It installs the extension and compiles its settings schema under your user account.
2. Log out and back in so GNOME discovers the extension.
3. Run `gnome-extensions enable chroma-panel@local`.
4. Reopen the panel. The status should read **Desktop filter · GNOME**. Turn on **Color filters** to apply the effect.

GNOME stores the desktop settings and continues applying them after the panel closes. You can disable the filter from the switch or unload the extension with `gnome-extensions disable chroma-panel@local`. Preview-only settings last for the current run.

The panel communicates with the extension through the session D-Bus. GNOME applies the shader once to the desktop, including the panel's own preview.
