CREATE TABLE IF NOT EXISTS demoCatalog.sandbox.orders_demo (
    id INT,
    user_id INT,
    created_at TIMESTAMP,
    total_amount DECIMAL(10, 2)
);

-- Use CURRENT_DATE so the "last 30 days" logic is relative to runtime.
INSERT INTO demoCatalog.sandbox.orders_demo VALUES
(101, 1, CURRENT_DATE - INTERVAL '10' DAY, 150.75),
(102, 1, CURRENT_DATE - INTERVAL '45' DAY, 200.00),
(103, 2, CURRENT_DATE - INTERVAL '5' DAY, 75.50),
(104, 3, CURRENT_DATE - INTERVAL '25' DAY, 310.20),
(105, 3, CURRENT_DATE - INTERVAL '90' DAY, 99.99);
