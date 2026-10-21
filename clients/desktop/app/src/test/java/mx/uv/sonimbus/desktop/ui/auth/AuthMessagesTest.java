package mx.uv.sonimbus.desktop.ui.auth;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;

import static org.junit.jupiter.api.Assertions.assertNull;

import java.io.IOException;
import java.time.Duration;
import java.util.EnumSet;
import java.util.List;
import java.util.Set;
import mx.uv.sonimbus.desktop.domain.session.RegistrationValidator;
import mx.uv.sonimbus.desktop.domain.session.RegistrationValidator.Problem;
import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.EnumSource;

class AuthMessagesTest {

    private static final String CONNECTION =
            "No se pudo conectar con el servidor. Revisa tu conexión e intenta de nuevo.";

    @Test
    void registerConflict() {
        assertEquals("Ya existe una cuenta con este correo.",
                AuthMessages.forRegister(http(ApiErrorKind.EMAIL_TAKEN, 409, null)));
    }

    @Test
    void registerValidation() {
        assertEquals("Revisa el correo y la contraseña.",
                AuthMessages.forRegister(http(ApiErrorKind.VALIDATION, 400, null)));
    }

    @Test
    void registerRateLimitedUsesRetryAfter() {
        assertEquals("Demasiadas solicitudes. Intenta de nuevo en 60 segundos.",
                AuthMessages.forRegister(http(ApiErrorKind.RATE_LIMITED, 429, Duration.ofSeconds(60))));
        assertEquals("Demasiadas solicitudes. Intenta de nuevo en 1 segundo.",
                AuthMessages.forRegister(http(ApiErrorKind.RATE_LIMITED, 429, Duration.ofSeconds(1))));
    }

    @Test
    void registerRateLimitedWithoutUsableWait() {
        String expected = "Demasiadas solicitudes. Intenta de nuevo en unos momentos.";
        assertEquals(expected, AuthMessages.forRegister(http(ApiErrorKind.RATE_LIMITED, 429, null)));
        assertEquals(expected, AuthMessages.forRegister(http(ApiErrorKind.RATE_LIMITED, 429, Duration.ZERO)));
    }

    @Test
    void registerConnectionProblems() {
        assertEquals(CONNECTION, AuthMessages.forRegister(noResponse(ApiErrorKind.NETWORK)));
        assertEquals(CONNECTION, AuthMessages.forRegister(noResponse(ApiErrorKind.TIMEOUT)));
        assertEquals(CONNECTION, AuthMessages.forRegister(http(ApiErrorKind.SERVER, 500, null)));
        assertEquals(CONNECTION, AuthMessages.forRegister(http(ApiErrorKind.UNAVAILABLE, 503, null)));
    }

    @Test
    void resendMessages() {
        assertEquals("Revisa el correo.",
                AuthMessages.forResendVerification(http(ApiErrorKind.VALIDATION, 400, null)));
        assertEquals("No pudimos enviar el correo de verificación. Intenta de nuevo más tarde.",
                AuthMessages.forResendVerification(http(ApiErrorKind.SERVER, 500, null)));
        assertEquals("Demasiadas solicitudes. Intenta de nuevo en 60 segundos.",
                AuthMessages.forResendVerification(
                        http(ApiErrorKind.RATE_LIMITED, 429, Duration.ofSeconds(60))));
        assertEquals(CONNECTION, AuthMessages.forResendVerification(noResponse(ApiErrorKind.TIMEOUT)));
    }

    /** Ningún mensaje deja ver el código, el caso interno ni texto en inglés. */
    @ParameterizedTest
    @EnumSource(ApiErrorKind.class)
    void messagesHaveNoInternals(ApiErrorKind kind) {
        for (String message : new String[] {
                AuthMessages.forRegister(http(kind, 418, null)),
                AuthMessages.forResendVerification(http(kind, 418, null))}) {
            assertFalse(message.contains("418"), message);
            assertFalse(message.contains(kind.name()), message);
            assertFalse(message.matches(".*\\b(request|failed|retry|invalid|please)\\b.*"), message);
        }
    }

    @Test
    void passwordRuleMatchesTheServer() {
        assertEquals("De 8 a 128 caracteres, con al menos una mayúscula, una minúscula, un número y un símbolo.",
                AuthMessages.PASSWORD_RULE);
    }

    @Test
    void emailFieldMessages() {
        assertEquals("Escribe tu correo electrónico.", AuthMessages.forEmail(Set.of(Problem.EMAIL_EMPTY)));
        assertEquals("El correo no tiene un formato válido.", AuthMessages.forEmail(Set.of(Problem.EMAIL_INVALID)));
        assertNull(AuthMessages.forEmail(Set.of(Problem.PASSWORD_EMPTY)));
    }

    @Test
    void passwordMessageListsEverythingMissing() {
        assertEquals("La contraseña necesita al menos 8 caracteres, una mayúscula y un símbolo (por ejemplo ! o #).",
                AuthMessages.forPassword(EnumSet.of(Problem.PASSWORD_TOO_SHORT,
                        Problem.PASSWORD_MISSING_UPPERCASE, Problem.PASSWORD_MISSING_SYMBOL)));
        assertEquals("La contraseña necesita un número.",
                AuthMessages.forPassword(Set.of(Problem.PASSWORD_MISSING_DIGIT)));
        assertEquals("La contraseña necesita una minúscula.",
                AuthMessages.forPassword(Set.of(Problem.PASSWORD_MISSING_LOWERCASE)));
        assertEquals("La contraseña puede tener como máximo 128 caracteres.",
                AuthMessages.forPassword(Set.of(Problem.PASSWORD_TOO_LONG)));
        assertEquals("Escribe una contraseña.", AuthMessages.forPassword(Set.of(Problem.PASSWORD_EMPTY)));
        assertNull(AuthMessages.forPassword(Set.of(Problem.CONFIRMATION_MISMATCH)));
    }

    @Test
    void passwordMessageFromRealValidation() {
        Set<Problem> problems = RegistrationValidator.validate("ana@uv.mx", "invalidpassword", "invalidpassword").problems();

        assertEquals("La contraseña necesita una mayúscula, un número y un símbolo (por ejemplo ! o #).",
                AuthMessages.forPassword(problems));
    }

    @Test
    void confirmationMessages() {
        assertEquals("Confirma tu contraseña.", AuthMessages.forConfirmation(Set.of(Problem.CONFIRMATION_EMPTY)));
        assertEquals("Las contraseñas no coinciden.", AuthMessages.forConfirmation(Set.of(Problem.CONFIRMATION_MISMATCH)));
        assertNull(AuthMessages.forConfirmation(Set.of()));
    }

    @Test
    void spanishJoin() {
        assertEquals("a", AuthMessages.joinSpanish(List.of("a")));
        assertEquals("a y b", AuthMessages.joinSpanish(List.of("a", "b")));
        assertEquals("a, b y c", AuthMessages.joinSpanish(List.of("a", "b", "c")));
    }

    @Test
    void unavailableNotices() {
        assertEquals("El inicio de sesión todavía no está disponible en el servidor.", AuthMessages.LOGIN_UNAVAILABLE);
        assertEquals("No pudimos enviar el correo de verificación. Usa Reenviar correo.",
                AuthMessages.VERIFICATION_NOT_SENT);
        assertEquals("Si la cuenta existe y no está verificada, te enviamos un nuevo enlace.", AuthMessages.RESEND_DONE);
    }

    @Test
    void cooldownCountdownText() {
        assertEquals("Reenviar en 42 s", VerifyAccountController.cooldownText(42));
        assertEquals(60, VerifyAccountController.RESEND_COOLDOWN_SECONDS);
    }

    private static ApiException http(ApiErrorKind kind, int status, Duration retryAfter) {
        return new ApiException(kind, "test", status, retryAfter);
    }

    private static ApiException noResponse(ApiErrorKind kind) {
        return new ApiException(kind, "test", new IOException("Connection refused"));
    }
}
