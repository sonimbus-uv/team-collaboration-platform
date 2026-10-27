-- =====================================================================
-- Esquemas y usuarios por servicio (D-13, T-04)
--
-- Una base de datos compartida; cada servicio tiene su propio esquema y su
-- propio usuario, con permisos solo sobre ese esquema.
--
-- Idempotente: puede ejecutarse varias veces sin efectos (init.sh lo aplica
-- en cada `docker compose up`). Las contraseñas se leen del entorno y se
-- resincronizan en cada ejecución con las del archivo .env.
--
-- Variables de entorno requeridas (psql 15 o posterior, \getenv):
--   AUTH_DB_PASSWORD, CORE_DB_PASSWORD, FILES_DB_PASSWORD, REPORTS_DB_PASSWORD
-- =====================================================================

\set ON_ERROR_STOP on

\getenv auth_password AUTH_DB_PASSWORD
\getenv core_password CORE_DB_PASSWORD
\getenv files_password FILES_DB_PASSWORD
\getenv reports_password REPORTS_DB_PASSWORD

-- Si una sentencia falla, PostgreSQL registra su texto en el log del servidor;
-- en esta sesión se desactiva para que un ALTER ROLE ... PASSWORD fallido no
-- deje la contraseña en el log. Requiere superusuario, como el resto del script.
SET log_min_error_statement = panic;
SET log_statement = 'none';

-- ---------------------------------------------------------------------
-- Base de datos: nadie se conecta salvo los usuarios autorizados abajo
-- ---------------------------------------------------------------------
SELECT format('REVOKE ALL ON DATABASE %I FROM PUBLIC', current_database()) \gexec

-- PostgreSQL 15+ ya no permite crear en public por defecto; se deja explícito.
REVOKE CREATE ON SCHEMA public FROM PUBLIC;

-- Extensiones compartidas. Viven en public y las instala el superusuario,
-- porque los usuarios de servicio no tienen CREATE sobre la base de datos.
-- Auth usa citext (correo sin distinguir mayúsculas).
CREATE EXTENSION IF NOT EXISTS citext WITH SCHEMA public;

-- ---------------------------------------------------------------------
-- Usuarios (uno por servicio)
-- ---------------------------------------------------------------------
SELECT format('CREATE ROLE %I LOGIN', r)
FROM unnest(ARRAY['auth_service', 'core_service', 'files_service', 'reports_service']) AS r
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = r) \gexec

ALTER ROLE auth_service    WITH LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT PASSWORD :'auth_password';
ALTER ROLE core_service    WITH LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT PASSWORD :'core_password';
ALTER ROLE files_service   WITH LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT PASSWORD :'files_password';
ALTER ROLE reports_service WITH LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT PASSWORD :'reports_password';

-- ---------------------------------------------------------------------
-- Esquemas, conexión y search_path
--
-- search_path = <esquema>, public:
--   * las tablas de migraciones (_sqlx_migrations de Auth, schema_migrations
--     de Ecto) se crean dentro del esquema del servicio;
--   * los tipos de las extensiones (citext) se resuelven desde public.
-- ---------------------------------------------------------------------
SELECT format('CREATE SCHEMA IF NOT EXISTS %I AUTHORIZATION %I', s, r)
     || format('; GRANT CONNECT, TEMPORARY ON DATABASE %I TO %I', current_database(), r)
     || format('; ALTER ROLE %I IN DATABASE %I SET search_path = %I, public', r, current_database(), s)
FROM (VALUES
    ('auth',    'auth_service'),
    ('core',    'core_service'),
    ('files',   'files_service'),
    ('reports', 'reports_service')
) AS t(s, r) \gexec

-- ---------------------------------------------------------------------
-- Excepción temporal: Auth
--
-- services/auth/migrations/0001_initial_auth_schema.sql ejecuta
-- `CREATE SCHEMA IF NOT EXISTS auth`, y PostgreSQL exige CREATE sobre la base
-- de datos antes de comprobar que el esquema ya existe. Sin este permiso la
-- migración falla con auth_service. Se aparta de D-13 y es temporal.
-- Condición de retiro: que Auth (@tuzc0) quite esa sentencia de su migración
-- (el esquema ya lo crea este script); en ese PR se borra este bloque y se
-- ejecuta REVOKE CREATE ON DATABASE ... FROM auth_service.
-- Efecto: auth_service puede crear esquemas nuevos, pero sigue sin poder leer
-- los de otros servicios.
-- ---------------------------------------------------------------------
SELECT format('GRANT CREATE ON DATABASE %I TO auth_service', current_database()) \gexec

-- ---------------------------------------------------------------------
-- Aviso para bases creadas antes de T-04
--
-- Si un esquema ya existía con otro dueño (por ejemplo, auth creado por la
-- migración de Auth con el superusuario), no se cambia aquí: sus tablas y su
-- historial de migraciones siguen siendo del superusuario. Ver
-- database/postgres/README.md, sección «Volúmenes anteriores a T-04».
-- ---------------------------------------------------------------------
DO $$
DECLARE
    item record;
BEGIN
    FOR item IN
        SELECT n.nspname, pg_get_userbyid(n.nspowner) AS owner, t.r
        FROM pg_namespace n
        JOIN (VALUES
            ('auth',    'auth_service'),
            ('core',    'core_service'),
            ('files',   'files_service'),
            ('reports', 'reports_service')
        ) AS t(s, r) ON t.s = n.nspname
        WHERE pg_get_userbyid(n.nspowner) <> t.r
    LOOP
        RAISE WARNING 'el esquema % pertenece a % y no a %; ver database/postgres/README.md',
            item.nspname, item.owner, item.r;
    END LOOP;
END
$$;
