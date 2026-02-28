with raw_events as (
    select
        user_id,
        event_name,
        event_ts,
        event_properties["screen"] as screen,
        event_properties["campaign"] as campaign
    from analytics_space."app_events_raw"
    where event_ts >= timestamp '2024-01-01 00:00:00'
),
sessions as (
    select
        user_id,
        event_ts,
        screen,
        campaign,
        sum(is_session_start) over (
            partition by user_id
            order by event_ts
            rows between unbounded preceding and current row
        ) as session_id
    from (
        select
            user_id,
            event_ts,
            screen,
            campaign,
            case
                when lag(event_ts) over (partition by user_id order by event_ts) is null then 1
                when event_ts - lag(event_ts) over (partition by user_id order by event_ts) > interval '30' minute then 1
                else 0
            end as is_session_start
        from raw_events
    ) x
),
session_level as (
    select
        user_id,
        session_id,
        min(event_ts) as session_start,
        max(event_ts) as session_end,
        count(*) as events_in_session,
        count(distinct screen) as screens_visited
    from sessions
    group by user_id, session_id
)
select
    user_id,
    session_id,
    session_start,
    session_end,
    events_in_session,
    screens_visited,
    (session_end - session_start) as session_duration
from session_level
where events_in_session >= 3
order by session_start desc
;
