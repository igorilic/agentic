pub struct Db {
    _pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
}
