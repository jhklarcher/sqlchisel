select  t.trip_distance , t.fare_amount, t.tip_amount, t.passenger_count
from Samples."samples.dremio.com"."NYC-taxi-trips" t
where  t.trip_distance > 5 and t.passenger_count >=2 and t.payment_type = 'CRD'
order by 1 desc , 2 desc
limit   25
;
