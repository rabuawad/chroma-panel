use zbus::blocking::{Connection, Proxy};

pub type Filter = (bool, u32, u32);

pub fn connect() -> Option<(Connection, Filter)> {
    let connection = Connection::session().ok()?;
    let state: Filter = proxy(&connection).ok()?.call("GetFilter", &()).ok()?;
    (state.1 < 3 && state.2 <= 100).then_some((connection, state))
}

fn proxy(connection: &Connection) -> zbus::Result<Proxy<'_>> {
    Proxy::new(
        connection,
        "org.gnome.Shell",
        "/local/ChromaPanel",
        "local.ChromaPanel",
    )
}

pub fn apply(connection: &Connection, state: Filter) -> zbus::Result<()> {
    proxy(connection)?.call("SetFilter", &state)
}
