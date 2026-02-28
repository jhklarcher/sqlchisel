use analytics_space."customer-metrics";

select  m.customer_id, m.metric_date, m.ltv_30d, m.ltv_90d,
        d.country, d.segment,
        case
            when m.ltv_90d >= 1000 then 'VIP'
            when m.ltv_90d >= 200 then 'REGULAR'
            else 'LOW'
        end as segment_bucket
from "ltv_daily"   m
left join analytics_space."dim_customers" d
    on d.customer_id = m.customer_id
where m.metric_date between date '2024-01-01' and date '2024-03-31'
and d.opt_out_marketing = false
order by m.metric_date desc, m.customer_id
;
