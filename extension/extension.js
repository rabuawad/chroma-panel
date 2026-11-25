import Cogl from 'gi://Cogl';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import Shell from 'gi://Shell';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const iface = `<node><interface name="local.ChromaPanel">
  <method name="GetFilter"><arg direction="out" type="b"/><arg direction="out" type="u"/><arg direction="out" type="u"/></method>
  <method name="SetFilter"><arg direction="in" type="b"/><arg direction="in" type="u"/><arg direction="in" type="u"/></method>
  <method name="GetFilterColor"><arg direction="out" type="b"/><arg direction="out" type="u"/><arg direction="out" type="u"/><arg direction="out" type="u"/></method>
  <method name="SetFilterColor"><arg direction="in" type="b"/><arg direction="in" type="u"/><arg direction="in" type="u"/><arg direction="in" type="u"/></method>
</interface></node>`;

const Filter = GObject.registerClass(class ChromaPanelEffect extends Shell.GLSLEffect {
    vfunc_build_pipeline() {
        this.add_glsl_snippet(Cogl.SnippetHook.FRAGMENT,
            'uniform float amount; uniform float mode; uniform vec3 tint;',
            `vec3 c = cogl_color_out.rgb;
             vec3 filtered;
             if (mode < 0.5)
                 filtered = vec3(dot(c, vec3(0.2126, 0.7152, 0.0722)));
             else if (mode < 1.5)
                 filtered = min(vec3(cogl_color_out.a), vec3(
                     dot(c, vec3(0.393, 0.769, 0.189)),
                     dot(c, vec3(0.349, 0.686, 0.168)),
                     dot(c, vec3(0.272, 0.534, 0.131))));
             else if (mode < 2.5)
                 filtered = vec3(cogl_color_out.a) - c;
             else
                 filtered = c * mix(vec3(0.35), vec3(1.0), tint);
             cogl_color_out.rgb = mix(c, filtered, amount);`, false);
    }
    update(mode, amount, color) {
        this.set_uniform_float(this.get_uniform_location('tint'), 3,
            [color >> 16, (color >> 8) & 255, color & 255].map(channel => channel / 255));
        this.set_uniform_float(this.get_uniform_location('mode'), 1, [mode]);
        this.set_uniform_float(this.get_uniform_location('amount'), 1, [amount / 100]);
        this.queue_repaint();
    }
});

export default class ChromaPanel extends Extension {
    enable() {
        this.settings = this.getSettings();
        this.effect = new Filter();
        global.stage.add_effect_with_name(this.uuid, this.effect);
        this.changed = this.settings.connect('changed', () => this.sync());
        this.sync();
        this.bus = Gio.DBusExportedObject.wrapJSObject(iface, this);
        this.bus.export(Gio.DBus.session, '/local/ChromaPanel');
    }
    GetFilter() {
        return this.settings.get_value('filter').deepUnpack();
    }
    SetFilter(enabled, mode, intensity) {
        if (mode > 2 || intensity > 100)
            throw new Error('Invalid color filter');
        this.SetFilterColor(enabled, mode, intensity, this.settings.get_uint('tint'));
    }
    GetFilterColor() {
        return [...this.GetFilter(), this.settings.get_uint('tint')];
    }
    SetFilterColor(enabled, mode, intensity, color) {
        if (mode > 3 || intensity > 100 || color > 0xffffff)
            throw new Error('Invalid color filter');
        this.settings.delay();
        this.settings.set_uint('tint', color);
        this.settings.set_value('filter', new GLib.Variant('(buu)', [enabled, mode, intensity]));
        this.settings.apply();
    }
    sync() {
        const [enabled, mode, intensity, color] = this.GetFilterColor();
        this.effect.set_enabled(enabled && intensity > 0);
        if (enabled && intensity > 0)
            this.effect.update(mode, intensity, color);
    }
    disable() {
        this.bus.unexport();
        this.settings.disconnect(this.changed);
        global.stage.remove_effect(this.effect);
        this.bus = this.settings = this.effect = null;
    }
}
