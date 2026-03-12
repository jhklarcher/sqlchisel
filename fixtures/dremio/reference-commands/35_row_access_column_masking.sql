ROW COLUMN POLICIES ON my_space.my_table;
ALTER TABLE my_space.my_table MODIFY COLUMN ssn SET MASKING POLICY mask_ssn (ssn);
