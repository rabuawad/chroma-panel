use zbus::blocking::{Connection, Proxy};

pub type Filter = (bool, u32, u32, u32);
pub const TINT: u32 = 3;

pub struct Desktop {
    connection: Connection,
    pub supports_tint: bool,
}

fn proxy(connection: &Connection) -> zbus::Result<Proxy<'_>> {
    Proxy::new(
        connection,
        "org.gnome.Shell",
        "/local/ChromaPanel",
        "local.ChromaPanel",
    )
}

impl Desktop {
    pub fn connect(default_color: u32) -> Option<(Self, Filter)> {
        let connection = Connection::session().ok()?;
        let service = proxy(&connection).ok()?;
        let (supports_tint, state) = match service.call::<_, _, Filter>("GetFilterColor", &()) {
            Ok(state) => (true, state),
            Err(_) => {
                let (enabled, mode, level): (bool, u32, u32) =
                    service.call("GetFilter", &()).ok()?;
                (false, (enabled, mode, level, default_color))
            }
        };
        drop(service);
        (state.1 <= TINT && state.2 <= 100 && state.3 <= 0xffffff).then_some((
            Self {
                connection,
                supports_tint,
            },
            state,
        ))
    }

    pub fn apply(&self, state: Filter) -> zbus::Result<()> {
        let service = proxy(&self.connection)?;
        if self.supports_tint {
            service.call("SetFilterColor", &state)
        } else {
            // The legacy compositor must release its effect for the local tint preview.
            let (enabled, mode, level, _) = state;
            service.call(
                "SetFilter",
                &(
                    enabled && mode != TINT,
                    if mode == TINT { 0 } else { mode },
                    level,
                ),
            )
        }
    }
}
