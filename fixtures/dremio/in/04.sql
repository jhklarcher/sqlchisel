select
    o.order_id, o.customer_id, o.order_ts, o.status,
    i.sku, i.quantity, i.extended_price,
    case when o.status in ('CANCELLED','RETURNED') then 1 else 0 end as is_problem
from arctic_catalog.analytics_space.orders_history o
at branch feature_campaign_x
as of timestamp '2024-10-01 12:00:00'
join arctic_catalog.analytics_space.order_items_history i
  on i.order_id = o.order_id
  and i.valid_from <= o.order_ts
  and (i.valid_to  is null or i.valid_to > o.order_ts)
where o.order_ts between timestamp '2024-09-01 00:00:00' and timestamp '2024-09-30 23:59:59'
and o.channel in ('web','mobile')
order by o.order_ts desc, o.order_id
;
