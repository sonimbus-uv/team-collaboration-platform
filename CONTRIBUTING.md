# Guía de Contribución

Este documento resume las reglas básicas que todos los integrantes del equipo deberán seguir al trabajar en el proyecto.

Para decisiones técnicas y normas detalladas deberá consultarse:

`docs/STANDARD_DEVELOPMENT.md`

---

## 1. Antes de comenzar

Antes de desarrollar una funcionalidad deberá existir:

- un caso de uso o requisito identificado en [`docs/requirements/`](docs/requirements/);
- un issue asociado;
- criterios de aceptación claros;
- una rama específica.

No deberá comenzarse una funcionalidad directamente sobre `main`.

---

## 2. Rama principal

La rama principal será:

```text
main
```

`main` deberá mantenerse estable y funcional.

No se deberán realizar commits directos sobre `main`, salvo casos excepcionales autorizados por el equipo.

---

## 3. Creación de ramas

Formato:

```text
feature/<modulo>-<descripcion>
fix/<modulo>-<descripcion>
tech/<descripcion>
refactor/<modulo>-<descripcion>
docs/<descripcion>
test/<modulo>-<descripcion>
```

Ejemplos:

```text
feature/auth-login
feature/core-groups-create
fix/auth-token-expiration
tech/docker-compose
tech/linux-vm
tech/postgresql-container
refactor/core-messages-repository
docs/update-api-auth
test/core-projects-service
```

`<modulo>` y el `alcance` de los commits deben ser uno de los alcances de la sección 5 de [`docs/STANDARD_DEVELOPMENT.md`](docs/STANDARD_DEVELOPMENT.md#alcances): `desktop`, `mobile`, `auth`, `core`, `files`, `reports`, `gateway`, `contracts`, `database`, `infrastructure` o `docs`. El dominio funcional va en la descripción (`feature/core-groups-create`), no en el alcance.

Las ramas deberán tener una vida corta y centrarse en una sola responsabilidad.

---

## 4. Commits

Se utilizará Conventional Commits.

Formato:

```text
tipo(alcance): descripción
```

Tipos:

```text
feat
fix
docs
test
refactor
chore
build
ci
perf
```

Ejemplos:

```text
feat(auth): add user registration
fix(auth): reject expired refresh token
test(core): add group creation tests
docs(contracts): document password recovery
refactor(core): extract project repository
```

Evitar:

```text
cambios
final
ya funciona
correccion2
prueba
aaa
```

---

## 5. Actualizar la rama antes del Pull Request

```bash
git checkout main
git pull origin main

git checkout feature/auth-login
git merge main
```

Los conflictos deberán resolverse antes de solicitar revisión.

---

## 6. Calidad del código

### Java

- compilación;
- pruebas;
- formateador común.

### Flutter / Dart

```bash
dart format .
flutter analyze
flutter test
```

### Rust

```bash
cargo fmt
cargo clippy
cargo test
```

### Elixir

```bash
mix format
mix test
```

Cuando se incorpore:

```text
Credo
```

---

## 7. Pruebas

Toda funcionalidad nueva deberá incluir pruebas cuando corresponda:

- unitarias;
- integración;
- contrato;
- end-to-end.

Las pruebas deberán validar comportamiento relevante.

---

## 8. Seguridad

No deberán agregarse al repositorio:

- contraseñas;
- tokens;
- API keys;
- private keys;
- credenciales de bases de datos;
- secrets.

La configuración sensible deberá almacenarse mediante variables de entorno o mecanismos equivalentes.

Los archivos `.env` reales deberán incluirse en `.gitignore`.

Podrá mantenerse:

```text
.env.example
```

sin valores sensibles.

---

## 9. Bases de datos

Los cambios de estructura deberán realizarse mediante mecanismos reproducibles:

- migraciones;
- scripts versionados;
- esquemas reproducibles.

No se deberá depender de modificaciones manuales no documentadas.

---

## 10. APIs

Todo cambio en una API deberá actualizar su documentación.

Las APIs REST deberán mantenerse documentadas mediante OpenAPI.

Los cambios incompatibles deberán comunicarse al equipo.

---

## 11. Docker

Cuando un cambio afecte un servicio contenedorizado deberá comprobarse:

```bash
docker compose build
```

y cuando corresponda:

```bash
docker compose up
```

Los Dockerfiles deberán mantenerse reproducibles y sin secretos.

---

## 12. Pull Requests

Toda Pull Request deberá incluir:

- descripción;
- issue o requisito asociado;
- cambios realizados;
- pruebas ejecutadas;
- evidencia cuando aplique;
- riesgos o limitaciones conocidas.

No deberá fusionarse si existen errores críticos en:

- compilación;
- pruebas;
- CI;
- seguridad;
- integración.

---

## 13. Code Review

El código deberá ser revisado por una persona distinta a quien lo desarrolló.

- `.github/CODEOWNERS` asigna automáticamente como revisores a `@Maple-M136279841` y `@tuzc0`, y `main` exige una aprobación de un codeowner.
- `@roluva` es miembro del equipo, pero todavía no es codeowner: sus PR los aprueba cualquiera de los dos, y sus revisiones en PR de otros son bienvenidas aunque no cuenten para la regla de `main`.
- Cada módulo tiene un responsable principal (ver [`docs/PLAN.md`](docs/PLAN.md#reparto-de-módulos)); un PR que toca el módulo de otra persona debe pedirle revisión.

Se deberá verificar:

- funcionalidad;
- mantenibilidad;
- claridad;
- seguridad;
- pruebas;
- impacto arquitectónico.

---

## 14. Merge

Solo podrá realizarse merge cuando:

- se cumplan los criterios de aceptación;
- las pruebas sean exitosas;
- la revisión esté aprobada;
- el CI sea exitoso;
- la documentación esté actualizada cuando corresponda.

Después del merge deberá eliminarse la rama si ya no es necesaria.

---

## 15. Definition of Done

Antes de marcar un issue como terminado deberá verificarse:

`docs/DEFINITION_OF_DONE.md`

Una funcionalidad no se considera terminada únicamente porque:

> “Funciona en mi computadora.”

---

## 16. Problemas encontrados

Si se detecta un problema fuera del alcance actual:

1. no deberá ignorarse;
2. no necesariamente deberá resolverse inmediatamente;
3. deberá registrarse como issue cuando sea relevante.

---

## 17. Decisiones arquitectónicas

Las decisiones importantes deberán documentarse mediante ADR.

No deberán modificarse decisiones arquitectónicas importantes únicamente dentro de una Pull Request sin documentar la razón.

---

## 18. Regla general

Antes de agregar una tecnología, framework, dependencia o servicio deberá responderse:

> ¿Qué problema concreto del proyecto resuelve?

Si no existe una respuesta clara, deberá evitarse incrementar la complejidad.
