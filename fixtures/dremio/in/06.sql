alter  reflection  analytics_space.daily_revenue_by_country_reflection
using
    display name 'Daily revenue by country reflection'
    partition by (order_date)
    distribute by (country)
    sort by (order_date, country)
;

create reflection analytics_space.daily_revenue_by_country_reflection2
using
    table arctic_catalog.analytics_space."daily_revenue_by_country"
    partition by (country)
    sort by (country, order_date)
;
