package mx.uv.sonimbus.desktop.domain.registration;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import mx.uv.sonimbus.desktop.domain.registration.RegistrationValidator.Problem;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationValidator.Result;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

import java.util.EnumSet;
import java.util.List;
import java.util.Set;

class RegistrationValidatorTest {

    private static final String EMAIL = "ana@uv.mx";
    private static final String VALID = "ValidPass123!";

    @Test
    void validFormHasNoProblems() {
        Result result = RegistrationValidator.validate(EMAIL, VALID, VALID);

        assertTrue(result.isValid(), result.problems().toString());
        assertEquals(EMAIL, result.email());
    }

    @Test
    void emailIsNormalizedLikeAuth() {
        assertEquals(
                "test@example.com",
                RegistrationValidator.validate(" TEST@Example.COM ", VALID, VALID).email());
    }


    @Test
    void sevenCharactersIsTooShortAndEightIsEnough() {
        assertEquals(Set.of(Problem.PASSWORD_TOO_SHORT), passwordProblems("Abcde1!"));
        assertEquals(Set.of(), passwordProblems("Abcdef1!"));
    }

    @Test
    void oneHundredTwentyEightIsTheMaximum() {
        String max = "A".repeat(125) + "a1!";
        String over = "A".repeat(126) + "a1!";

        assertEquals(128, max.length());
        assertEquals(Set.of(), passwordProblems(max));
        assertEquals(Set.of(Problem.PASSWORD_TOO_LONG), passwordProblems(over));
    }

    @Test
    void lengthCountsCodePointsNotUtf16Units() {
        String sevenCodePoints = "Abcd1!😀";
        assertEquals(8, sevenCodePoints.length());
        assertEquals(Set.of(Problem.PASSWORD_TOO_SHORT), passwordProblems(sevenCodePoints));
    }


    @Test
    void missingUppercase() {
        assertEquals(Set.of(Problem.PASSWORD_MISSING_UPPERCASE), passwordProblems("validpass123!"));
    }

    @Test
    void missingLowercase() {
        assertEquals(Set.of(Problem.PASSWORD_MISSING_LOWERCASE), passwordProblems("VALIDPASS123!"));
    }

    @Test
    void missingDigit() {
        assertEquals(Set.of(Problem.PASSWORD_MISSING_DIGIT), passwordProblems("ValidPass!!!"));
    }

    @Test
    void missingSymbol() {
        assertEquals(Set.of(Problem.PASSWORD_MISSING_SYMBOL), passwordProblems("ValidPass123"));
    }

    @Test
    void allRequirementsAreReportedTogether() {
        assertEquals(
                EnumSet.of(
                        Problem.PASSWORD_MISSING_UPPERCASE,
                        Problem.PASSWORD_MISSING_DIGIT,
                        Problem.PASSWORD_MISSING_SYMBOL),
                passwordProblems("invalidpassword"));
        assertEquals(
                EnumSet.of(
                        Problem.PASSWORD_TOO_SHORT,
                        Problem.PASSWORD_MISSING_UPPERCASE,
                        Problem.PASSWORD_MISSING_DIGIT,
                        Problem.PASSWORD_MISSING_SYMBOL),
                passwordProblems("abc"));
    }

    @Test
    void spaceCountsAsSymbol() {
        assertEquals(Set.of(), passwordProblems("Valid Pass 123"));
    }

    @Test
    void unicodeLettersCountAsUpperAndLowerCase() {
        assertEquals(Set.of(), passwordProblems("ÁRBOLñandú1!"));
    }

    @Test
    void onlyAsciiDigitsCount() {
        assertEquals(EnumSet.of(Problem.PASSWORD_MISSING_DIGIT), passwordProblems("ValidPass٣!"));
    }

    @Test
    void numbersOutsideNdAreNotSymbolsAsInRust() {
        assertFalse(Character.isLetterOrDigit('²'));
        assertTrue(RegistrationValidator.isRustAlphanumeric('²'));
        assertTrue(RegistrationValidator.isRustAlphanumeric('Ⅻ'));
        assertEquals(Set.of(Problem.PASSWORD_MISSING_SYMBOL), passwordProblems("ValidPass1²Ⅻ"));
    }

    @Test
    void passwordIsNotTrimmed() {
        assertEquals(Set.of(Problem.PASSWORD_TOO_SHORT), passwordProblems("  Ab1!  ".strip()));
        assertEquals(Set.of(), passwordProblems("  Ab1!  "));
    }


    @ParameterizedTest
    @ValueSource(
            strings = {
                "invalid-email",
                "a@b",
                "a@@b.com",
                "a@b@c.com",
                "@b.com",
                "a@.com",
                "a@b.com.",
                "a@b..com",
                "a@-b.com",
                "a@b-.com",
                ".a@b.com",
                "a.@b.com",
                "a..b@b.com",
                "a@b_com.com",
                "a\u0000@b.com",
                "a b@c.com",
                "ñ@b.com",
                "a@ñ.com"
            })
    void rejectsMalformedAddresses(String email) {
        assertEquals(Set.of(Problem.EMAIL_INVALID), emailProblems(email), email);
    }

    @ParameterizedTest
    @ValueSource(strings = {"name+tag@sub.example.com", "a@b.co", "o'brien@uv.mx", "x-y_z@a-b.mx"})
    void acceptsValidAddresses(String email) {
        assertEquals(Set.of(), emailProblems(email), email);
    }

    @Test
    void emailLengthLimits() {
        String local64 = "a".repeat(64);
        assertEquals(Set.of(), emailProblems(local64 + "@uv.mx"));
        assertEquals(Set.of(Problem.EMAIL_INVALID), emailProblems(local64 + "a@uv.mx"));

        String label63 = "d".repeat(63);
        assertEquals(Set.of(), emailProblems("a@" + label63 + ".mx"));
        assertEquals(Set.of(Problem.EMAIL_INVALID), emailProblems("a@" + label63 + "d.mx"));

        String domain =
                String.join(
                        ".",
                        List.of("d".repeat(63), "d".repeat(63), "d".repeat(63), "d".repeat(60)));
        assertEquals(254, ("a@" + domain).length());
        assertEquals(Set.of(), emailProblems("a@" + domain));
        assertEquals(Set.of(Problem.EMAIL_INVALID), emailProblems("ab@" + domain));
    }

    @Test
    void trimUsesUnicodeWhiteSpaceLikeRust() {
        assertEquals(EMAIL, RegistrationValidator.normalizeEmail(" " + EMAIL + " "));
        assertEquals(Set.of(), emailProblems(" " + EMAIL));
    }

    @Test
    void kelvinSignLowercasesToAsciiLikeRust() {
        assertEquals("kim@uv.mx", RegistrationValidator.normalizeEmail("Kim@uv.mx"));
        assertEquals(Set.of(), emailProblems("Kim@uv.mx"));
    }


    @Test
    void emptyFormReportsEachField() {
        assertEquals(
                EnumSet.of(Problem.EMAIL_EMPTY, Problem.PASSWORD_EMPTY, Problem.CONFIRMATION_EMPTY),
                RegistrationValidator.validate("  ", "", "").problems());
        assertEquals(
                EnumSet.of(Problem.EMAIL_EMPTY, Problem.PASSWORD_EMPTY, Problem.CONFIRMATION_EMPTY),
                RegistrationValidator.validate(null, null, null).problems());
    }

    @Test
    void confirmationMustMatch() {
        assertEquals(
                Set.of(Problem.CONFIRMATION_MISMATCH),
                RegistrationValidator.validate(EMAIL, VALID, VALID + " ").problems());
        assertEquals(
                Set.of(Problem.CONFIRMATION_MISMATCH),
                RegistrationValidator.validate(EMAIL, VALID, "validpass123!").problems());
    }

    @Test
    void problemsKnowTheirField() {
        assertTrue(Problem.EMAIL_INVALID.concernsEmail());
        assertTrue(Problem.PASSWORD_MISSING_SYMBOL.concernsPassword());
        assertFalse(Problem.CONFIRMATION_MISMATCH.concernsPassword());
        assertTrue(Problem.CONFIRMATION_MISMATCH.concernsConfirmation());
    }

    private static Set<Problem> passwordProblems(String password) {
        return RegistrationValidator.validate(EMAIL, password, password).problems();
    }

    private static Set<Problem> emailProblems(String email) {
        return RegistrationValidator.validate(email, VALID, VALID).problems();
    }
}
