use super::{models::MarketRow, DbPool};
use chrono::{DateTime, Utc};

pub async fn upsert_market(pool: &DbPool, market: &MarketRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        insert into markets (id, question, category, created_ts, resolution_ts, status)
        values ($1,$2,$3,$4,$5,$6)
        on conflict (id) do update set question = excluded.question,
            category = excluded.category,
            created_ts = excluded.created_ts,
            resolution_ts = excluded.resolution_ts,
            status = excluded.status
        "#,
    )
    .bind(&market.id)
    .bind(&market.question)
    .bind(&market.category)
    .bind(market.created_ts)
    .bind(market.resolution_ts)
    .bind(&market.status)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(())
}

pub async fn recent_markets(
    pool: &DbPool,
    since: DateTime<Utc>,
) -> Result<Vec<MarketRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, MarketRow>(
        "select id, question, category, created_ts, resolution_ts, status from markets where created_ts > $1",
    )
    .bind(since)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
