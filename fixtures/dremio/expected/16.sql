ALTER TABLE demoCatalog.sales.staging."flight_segments"
CREATE RAW REFLECTION reporting
USING DISPLAY (
  flight_id,
  leg_number,
  departure_airport_id,
  arrival_airport_id,
  marketing_carrier_id,
  operating_carrier_id,
  departure_time_utc,
  arrival_time_utc,
  cabin_class_code,
  baggage_allowance_kg
);

ALTER TABLE demoCatalog.sales.staging."order_items"
CREATE RAW REFLECTION reporting
USING DISPLAY (
  order_id,
  product_type_id,
  product_item_id,
  revenue_amount
);

ALTER TABLE demoCatalog.sales.staging.cart_sessions
CREATE RAW REFLECTION reporting
USING DISPLAY (
  cart_id,
  site_id,
  created_at,
  cart_value,
  first_service_date
)
PARTITION BY (MONTH(created_at));