ALTER TABLE users
ALTER COLUMN password TYPE VARCHAR(256),
ALTER COLUMN password SET NOT NULL;
