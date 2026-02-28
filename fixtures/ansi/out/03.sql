SELECT *
FROM TABLE (external_query('postgres_orders', 'select o.id as order_id, o.customer_id, o.created_at, o.amount, c.country
         from public.orders o
         join public.customers c on c.id = o.customer_id
         where o.created_at >= timestamp ''2024-01-01 00:00:00''
         and o.amount > 0
         order by o.created_at desc
         limit 500')) AS ext_orders
WHERE ext_orders.country IN ('' SE '', '' NO '', '' DK '');