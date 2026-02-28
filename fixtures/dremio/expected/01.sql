SELECT
  t.trip_distance,
  t.fare_amount,
  t.tip_amount,
  t.passenger_count
FROM Samples."samples.dremio.com"."NYC-taxi-trips" t
WHERE t.trip_distance > 5
  AND t.passenger_count >= 2
  AND t.payment_type = 'CRD'
ORDER BY 1 DESC, 2 DESC
LIMIT 25;