import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import Clutter from 'gi://Clutter';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const BUS_NAME = 'io.github.alexriderspy.Ryshift';
const OBJECT_PATH = '/io/github/alexriderspy/Ryshift';
const INTERFACE = 'io.github.alexriderspy.Ryshift';

const WHEEL_TITLE = 'Ryshift';
const WHEEL_SIZE = 320;
const DRAG_THRESHOLD = 6;
const POLL_MS = 16;

function freshDrag() {
    return {down: false, startX: 0, startY: 0, moved: false, shown: false, shift: false};
}

export default class RyshiftExtension extends Extension {
    enable() {
        this._settings = this.getSettings();
        this._drag = freshDrag();
        this._pollId = 0;
        this._pending = null;
        this._wheel = null;

        this._nameWatchId = Gio.bus_watch_name(
            Gio.BusType.SESSION, BUS_NAME, Gio.BusNameWatcherFlags.NONE,
            () => this._startPolling(),
            () => this._stopPolling());

        this._windowCreatedId = global.display.connect('window-created',
            (_display, window) => this._watchWindow(window));

        if (Meta.is_wayland_compositor()) {
            Main.wm.addKeybinding('open-wheel', this._settings, Meta.KeyBindingFlags.NONE,
                Shell.ActionMode.NORMAL | Shell.ActionMode.OVERVIEW,
                () => this._openFromShortcut());
        }
    }

    disable() {
        if (Meta.is_wayland_compositor())
            Main.wm.removeKeybinding('open-wheel');
        this._stopPolling();
        if (this._nameWatchId) {
            Gio.bus_unwatch_name(this._nameWatchId);
            this._nameWatchId = 0;
        }
        if (this._windowCreatedId) {
            global.display.disconnect(this._windowCreatedId);
            this._windowCreatedId = 0;
        }
        this._settings = null;
        this._pending = null;
        this._wheel = null;
    }

    _startPolling() {
        if (!Meta.is_wayland_compositor() || this._pollId)
            return;
        this._drag = freshDrag();
        this._pollId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, POLL_MS, () => {
            this._tick();
            return GLib.SOURCE_CONTINUE;
        });
    }

    _stopPolling() {
        if (this._pollId) {
            GLib.source_remove(this._pollId);
            this._pollId = 0;
        }
    }

    _tick() {
        const [x, y, mods] = global.get_pointer();
        const down = (mods & Clutter.ModifierType.BUTTON1_MASK) !== 0;
        const shift = (mods & Clutter.ModifierType.SHIFT_MASK) !== 0;
        const d = this._drag;

        if (down && !d.down) {
            this._drag = {...freshDrag(), down: true, startX: x, startY: y, shift};
            return;
        }
        if (!down && d.down) {
            if (d.shown)
                this._call('MouseUp');
            this._drag = freshDrag();
            return;
        }
        if (d.down && !d.moved)
            d.moved = Math.abs(x - d.startX) > DRAG_THRESHOLD || Math.abs(y - d.startY) > DRAG_THRESHOLD;

        if (d.moved && shift && !d.shift) {
            if (d.shown) {
                this._call('Cycle');
            } else {
                d.shown = true;
                this._showAt(x, y, 'drag');
            }
        }
        d.shift = shift;
    }

    _openFromShortcut() {
        const [x, y] = global.get_pointer();
        this._showAt(x, y, 'shortcut');
    }

    _showAt(x, y, mode) {
        this._pending = {x, y, focus: mode === 'shortcut'};
        this._call('Show', new GLib.Variant('(dds)', [x, y, mode]));
        this._place();
    }

    _isWheel(window) {
        return window.get_title() === WHEEL_TITLE &&
            (window.get_wm_class() ?? '').toLowerCase().includes('ryshift');
    }

    _watchWindow(window) {
        const shownId = window.connect('shown', () => {
            window.disconnect(shownId);
            if (!this._isWheel(window))
                return;
            this._wheel = window;
            window.connect('unmanaged', () => {
                if (this._wheel === window)
                    this._wheel = null;
            });
            this._place();
        });
    }

    _place() {
        const window = this._wheel;
        const target = this._pending;
        if (!window || !target)
            return;
        const half = WHEEL_SIZE / 2;
        window.move_frame(true, Math.round(target.x - half), Math.round(target.y - half));
        window.make_above();
        if (!window.is_on_all_workspaces())
            window.stick();
        if (target.focus)
            window.activate(global.get_current_time());
        this._pending = null;
    }

    _call(method, params = null) {
        Gio.DBus.session.call(
            BUS_NAME, OBJECT_PATH, INTERFACE, method, params, null,
            Gio.DBusCallFlags.NO_AUTO_START, -1, null,
            (connection, result) => {
                try {
                    connection.call_finish(result);
                } catch (e) {
                    console.warn(`Ryshift: ${method} failed: ${e.message}`);
                }
            });
    }
}
