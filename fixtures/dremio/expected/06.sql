ALTER REFLECTION
  analytics_space.daily_revenue_by_country_reflection
  USING
    DISPLAY NAME 'Daily revenue by country reflection'
    PARTITION BY (order_date)
    DISTRIBUTE BY (country)
    SORT BY (order_date, country);

CREATE REFLECTION
  analytics_space.daily_revenue_by_country_reflection2
  USING
    TABLE arctic_catalog.analytics_space."daily_revenue_by_country"
    PARTITION BY (country)
    SORT BY (country, order_date);