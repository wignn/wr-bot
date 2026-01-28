#!/bin/bash
set -e

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    -- Create readonly user
    CREATE USER readonly WITH PASSWORD 'readonly';
    
    -- Grant connect permission
    GRANT CONNECT ON DATABASE wrbot TO readonly;
    
    -- Grant usage on public schema
    GRANT USAGE ON SCHEMA public TO readonly;
    
    -- Grant SELECT on all existing tables
    GRANT SELECT ON ALL TABLES IN SCHEMA public TO readonly;
    
    -- Grant SELECT on all future tables
    ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO readonly;
    
    -- Grant SELECT on all existing sequences
    GRANT SELECT ON ALL SEQUENCES IN SCHEMA public TO readonly;
EOSQL

echo "Read-only user 'readonly' created successfully!"
