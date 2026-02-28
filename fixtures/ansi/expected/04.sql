WITH raw_events AS (
  SELECT
    user_id,
    event_name,
    event_ts,
    event_properties["screen"] AS screen,
    event_properties["campaign"] AS campaign
  FROM analytics_space."app_events_raw"
  WHERE event_ts >= TIMESTAMP '2024-01-01 00:00:00'
),
sessions AS (
  SELECT
    user_id,
    event_ts,
    screen,
    campaign,
    SUM(is_session_start) OVER (
      PARTITION BY user_id
      ORDER BY event_ts
      ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
    ) AS session_id
  FROM (
      SELECT
        user_id,
        event_ts,
        screen,
        campaign,
        CASE
          WHEN LAG(event_ts) OVER (
            PARTITION BY user_id
            ORDER BY event_ts
          ) IS NULL THEN 1
          WHEN event_ts - LAG(event_ts) OVER (
            PARTITION BY user_id
            ORDER BY event_ts
          ) > INTERVAL '30' MINUTE THEN 1
          ELSE 0
        END AS is_session_start
      FROM raw_events
    ) x
),
session_level AS (
  SELECT
    user_id,
    session_id,
    MIN(event_ts) AS session_start,
    MAX(event_ts) AS session_end,
    COUNT(*) AS events_in_session,
    COUNT(DISTINCT screen) AS screens_visited
  FROM sessions
  GROUP BY
    user_id,
    session_id
)

SELECT
  user_id,
  session_id,
  session_start,
  session_end,
  events_in_session,
  screens_visited,
  (session_end - session_start) AS session_duration
FROM session_level
WHERE events_in_session >= 3
ORDER BY session_start DESC;