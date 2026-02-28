-- Top-level header comment
-- Describes the customer dimension

SELECT
    id,          -- customer id
    full_name,   -- display name
    country_code -- ISO country code
FROM dim_customers;

/*
 Multi-line comment describing the staging table
*/
CREATE TABLE staging.customers AS
SELECT * FROM dim_customers;
