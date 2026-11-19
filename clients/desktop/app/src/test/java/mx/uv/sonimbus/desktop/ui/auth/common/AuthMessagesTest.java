package mx.uv.sonimbus.desktop.ui.auth.common;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;

import mx.uv.sonimbus.desktop.domain.registration.RegistrationError;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationException;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationValidator;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationValidator.Problem;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.EnumSource;

import java.io.IOException;
import java.time.Duration;
import java.util.EnumSet;
import java.util.List;
import java.util.Set;

class AuthMessagesTest {

    private static final String CONNECTION =
            "No se pudo conectar con el servidor. Revisa tu conexión e intenta de nuevo.";

    @Test
    void registerConflict() {
        assertEquals(
                "Ya existe una cuenta con este correo.",
                AuthMessages.forRegister(failure(RegistrationError.EMAIL_TAKEN, null)));
    }

    @Test
    void registerValidation() {
        assertEquals(
                "Revisa el correo y la contraseña.",
                AuthMessages.forRegister(failure(RegistrationError.VALIDATION, null)));
    }

    @Test
    void registerRateLimitedUsesRetryAfter() {
        assertEquals(
                "Demasiadas solicitudes. Intenta de nuevo en 60 segundos.",
                AuthMessages.forRegister(
                        failure(RegistrationError.RATE_LIMITED, Duration.ofSeconds(60))));
        assertEquals(
                "Demasiadas solicitudes. Intenta de nuevo en 1 segundo.",
                AuthMessages.forRegister(
                        failure(RegistrationError.RATE_LIMITED, Duration.ofSeconds(1))));
    }

    @Test
    void registerRateLimitedWithoutUsableWait() {
        String expected = "Demasiadas solicitudes. Intenta de nuevo en unos momentos.";
        assertEquals(
                expected, AuthMessages.forRegister(failure(RegistrationError.RATE_LIMITED, null)));
        assertEquals(
                expected,
                AuthMessages.forRegister(failure(RegistrationError.RATE_LIMITED, Duration.ZERO)));
    }

    @Test
    void registerConnectionProblems() {
        assertEquals(CONNECTION, AuthMessages.forRegister(noResponse(RegistrationError.NETWORK)));
        assertEquals(CONNECTION, AuthMessages.forRegister(noResponse(RegistrationError.TIMEOUT)));
        assertEquals(CONNECTION, AuthMessages.forRegister(failure(RegistrationError.SERVER, null)));
        assertEquals(
                CONNECTION, AuthMessages.forRegister(failure(RegistrationError.UNAVAILABLE, null)));
    }

    @Test
    void resendMessages() {
        assertEquals(
                AuthMessages.INVALID_CODE,
                AuthMessages.forResendVerification(failure(RegistrationError.VALIDATION, null)));
        assertEquals(
                "No pudimos enviar el correo de verificación. Intenta de nuevo más tarde.",
                AuthMessages.forResendVerification(failure(RegistrationError.SERVER, null)));
        assertEquals(
                "Demasiadas solicitudes. Intenta de nuevo en 60 segundos.",
                AuthMessages.forResendVerification(
                        failure(RegistrationError.RATE_LIMITED, Duration.ofSeconds(60))));
        assertEquals(
                CONNECTION,
                AuthMessages.forResendVerification(noResponse(RegistrationError.TIMEOUT)));
    }

    /** Ningún mensaje deja ver el código, el caso interno ni texto en inglés. */
    @ParameterizedTest
    @EnumSource(RegistrationError.class)
    void messagesHaveNoInternals(RegistrationError kind) {
        for (String message :
                new String[] {
                    AuthMessages.forRegister(failure(kind, null)),
                    AuthMessages.forResendVerification(failure(kind, null))
                }) {
            assertFalse(message.contains("418"), message);
            assertFalse(message.contains(kind.name()), message);
            assertFalse(
                    message.matches(".*\\b(request|failed|retry|invalid|please)\\b.*"), message);
        }
    }

    @Test
    void passwordRuleMatchesTheServer() {
        assertEquals(
                "De 8 a 128 caracteres, con al menos una mayúscula, una minúscula, un número y un"
                        + " símbolo.",
                AuthMessages.PASSWORD_RULE);
    }

    @Test
    void emailFieldMessages() {
        assertEquals(
                "Escribe tu correo electrónico.",
                AuthMessages.forEmail(Set.of(Problem.EMAIL_EMPTY)));
        assertEquals(
                "El correo no tiene un formato válido.",
                AuthMessages.forEmail(Set.of(Problem.EMAIL_INVALID)));
        assertNull(AuthMessages.forEmail(Set.of(Problem.PASSWORD_EMPTY)));
    }

    @Test
    void passwordMessageListsEverythingMissing() {
        assertEquals(
                "La contraseña necesita al menos 8 caracteres, una mayúscula y un símbolo (por"
                        + " ejemplo ! o #).",
                AuthMessages.forPassword(
                        EnumSet.of(
                                Problem.PASSWORD_TOO_SHORT,
                                Problem.PASSWORD_MISSING_UPPERCASE,
                                Problem.PASSWORD_MISSING_SYMBOL)));
        assertEquals(
                "La contraseña necesita un número.",
                AuthMessages.forPassword(Set.of(Problem.PASSWORD_MISSING_DIGIT)));
        assertEquals(
                "La contraseña necesita una minúscula.",
                AuthMessages.forPassword(Set.of(Problem.PASSWORD_MISSING_LOWERCASE)));
        assertEquals(
                "La contraseña puede tener como máximo 128 caracteres.",
                AuthMessages.forPassword(Set.of(Problem.PASSWORD_TOO_LONG)));
        assertEquals(
                "Escribe una contraseña.",
                AuthMessages.forPassword(Set.of(Problem.PASSWORD_EMPTY)));
        assertNull(AuthMessages.forPassword(Set.of(Problem.CONFIRMATION_MISMATCH)));
    }

    @Test
    void passwordMessageFromRealValidation() {
        Set<Problem> problems =
                RegistrationValidator.validate("ana@uv.mx", "invalidpassword", "invalidpassword")
                        .problems();

        assertEquals(
                "La contraseña necesita una mayúscula, un número y un símbolo (por ejemplo ! o #).",
                AuthMessages.forPassword(problems));
    }

    @Test
    void confirmationMessages() {
        assertEquals(
                "Confirma tu contraseña.",
                AuthMessages.forConfirmation(Set.of(Problem.CONFIRMATION_EMPTY)));
        assertEquals(
                "Las contraseñas no coinciden.",
                AuthMessages.forConfirmation(Set.of(Problem.CONFIRMATION_MISMATCH)));
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
        assertEquals("Correo o contraseña incorrectos.", AuthMessages.INVALID_LOGIN);
        assertEquals(
                "No pudimos enviar el correo de verificación. Usa Reenviar correo.",
                AuthMessages.VERIFICATION_NOT_SENT);
        assertEquals(
                "Enviamos un nuevo código. Usa el más reciente; el plazo original no cambia.",
                AuthMessages.RESEND_DONE);
    }

    private static RegistrationException failure(RegistrationError kind, Duration retryAfter) {
        return new RegistrationException(kind, retryAfter, Set.of(), null);
    }

    private static RegistrationException noResponse(RegistrationError kind) {
        return new RegistrationException(
                kind, null, Set.of(), new IOException("Connection refused"));
    }
}
