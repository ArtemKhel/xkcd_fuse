use chrono::DateTime;
use log::info;
use rusqlite::{Connection, params};

use crate::xkcd::Xkcd;

pub fn db_init(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        r#"
        create table if not exists xkcds (
            num integer primary key,
            title text not null,
            safe_title text not null,
            image_url text not null,
            alt text not null,
            transcript text,
            link text,
            release_date integer not null
        )"#,
        [],
    )?;

    conn.execute(
        r#"
        create table if not exists images (
            num integer primary key,
            image_size integer not null,
            image_data blob not null,
            foreign key (num) references xkcds (num)
        )"#,
        [],
    )?;

    Ok(())
}
pub fn insert_meta(conn: &Connection, xkcd: &Xkcd) -> rusqlite::Result<()> {
    info!("Inserting xkcd {}", xkcd);
    let release_date = xkcd.release_date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
    conn.execute(
        r#"
        INSERT INTO xkcds (num, title, safe_title, image_url, alt, transcript, link, release_date)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(num) DO UPDATE SET
            title = excluded.title,
            safe_title = excluded.safe_title,
            image_url = excluded.image_url,
            alt = excluded.alt,
            transcript = excluded.transcript,
            link = excluded.link,
            release_date = excluded.release_date;
        "#,
        params![
            xkcd.num,
            xkcd.title,
            xkcd.safe_title,
            xkcd.image_url,
            xkcd.alt,
            xkcd.transcript,
            xkcd.link,
            release_date,
        ],
    )?;
    Ok(())
}

pub fn insert_image(conn: &Connection, num: u32, image_data: &[u8]) -> rusqlite::Result<()> {
    info!("Inserting image for xkcd {}", num);
    conn.execute(
        r#"
        INSERT INTO images (num, image_data, image_size)
        VALUES (?1, ?2, length(?2))
        ON CONFLICT(num) DO UPDATE SET
            image_data = excluded.image_data;
            image_size = legth(excluded.image_size);
        "#,
        params![num, image_data],
    )?;
    Ok(())
}

pub fn get_meta(conn: &Connection, num: u32) -> anyhow::Result<Xkcd> {
    info!("Loading from DB xkcd {}", num);
    let mut stmt = conn.prepare(
        r#"SELECT num, title, safe_title, image_url, alt, transcript, link, release_date FROM xkcds WHERE num = ?1"#,
    )?;

    let xkcd = stmt.query_row(params![num], |row| {
        let release_date = row.get::<_, i64>(7)?;
        let release_date = DateTime::from_timestamp(release_date, 0).unwrap().date_naive();
        Ok(Xkcd {
            num: row.get(0)?,
            title: row.get(1)?,
            safe_title: row.get(2)?,
            image_url: row.get(3)?,
            alt: row.get(4)?,
            transcript: row.get(5)?,
            link: row.get(6)?,
            release_date,
        })
    })?;

    Ok(xkcd)
}

pub fn get_image(conn: &Connection, num: u32) -> anyhow::Result<Vec<u8>> {
    info!("Loading from DB image for xkcd {}", num);
    let mut stmt = conn.prepare(r#"SELECT image_data FROM images wherE num = ?1"#)?;
    let image_data = stmt.query_row(params![num], |row| row.get::<_, Vec<u8>>(0))?;
    Ok(image_data)
}

pub fn get_image_size(conn: &Connection, num: u32) -> anyhow::Result<u64> {
    info!("Loading from DB image size for xkcd {}", num);
    let mut stmt = conn.prepare(r#"SELECT image_size FROM images WHERE num = ?1"#)?;
    let image_size = stmt.query_row(params![num], |row| row.get::<_, u64>(0))?;
    Ok(image_size)
}

pub fn get_stored_ids(conn: &Connection) -> anyhow::Result<Vec<u32>> {
    info!("Loading from DB all xkcd ids");
    let mut stmt = conn.prepare(r#"SELECT num FROM xkcds"#)?;
    let ids = stmt.query_map([], |row| row.get(0))?;
    let ids: Vec<u32> = ids.collect::<Result<_, _>>()?;
    Ok(ids)
}
