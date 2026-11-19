package mx.uv.sonimbus.desktop.domain.registration;

import java.nio.charset.StandardCharsets;
import java.util.EnumSet;
import java.util.Locale;
import java.util.Set;

public final class RegistrationValidator {
    public static final int PASSWORD_MIN_LENGTH = 8;
    public static final int PASSWORD_MAX_LENGTH = 128;

    private static final int EMAIL_MAX_BYTES = 254;
    private static final int LOCAL_MAX_BYTES = 64;
    private static final int LABEL_MAX_BYTES = 63;
    private static final String LOCAL_SPECIALS = ".!#$%&'*+-/=?^_`{|}~";

    public enum Problem {
        EMAIL_EMPTY,
        EMAIL_INVALID,
        PASSWORD_EMPTY,
        PASSWORD_TOO_SHORT,
        PASSWORD_TOO_LONG,
        PASSWORD_MISSING_UPPERCASE,
        PASSWORD_MISSING_LOWERCASE,
        PASSWORD_MISSING_DIGIT,
        PASSWORD_MISSING_SYMBOL,
        CONFIRMATION_EMPTY,
        CONFIRMATION_MISMATCH;

        public boolean concernsEmail() {
            return this == EMAIL_EMPTY || this == EMAIL_INVALID;
        }

        public boolean concernsPassword() {
            return name().startsWith("PASSWORD_");
        }

        public boolean concernsConfirmation() {
            return this == CONFIRMATION_EMPTY || this == CONFIRMATION_MISMATCH;
        }
    }

    public record Result(String email, Set<Problem> problems) {
        public boolean isValid() {
            return problems.isEmpty();
        }
    }

    private RegistrationValidator() {}

    public static Result validate(String email, String password, String confirmation) {

        String normalizedEmail = normalizeEmail(email == null ? "" : email);
        String passwordValue = password == null ? "" : password;
        String confirmationValue = confirmation == null ? "" : confirmation;
        Set<Problem> problems = EnumSet.noneOf(Problem.class);

        if (normalizedEmail.isEmpty()) {
            problems.add(Problem.EMAIL_EMPTY);
        } else if (!isValidEmailFormat(normalizedEmail)) {
            problems.add(Problem.EMAIL_INVALID);
        }

        problems.addAll(validatePassword(passwordValue, confirmationValue));

        return new Result(normalizedEmail, problems);
    }

    public static Set<Problem> validatePassword(String password, String confirmation) {
        String passwordValue = password == null ? "" : password;
        String confirmationValue = confirmation == null ? "" : confirmation;
        Set<Problem> problems = EnumSet.noneOf(Problem.class);

        if (passwordValue.isEmpty()) {
            problems.add(Problem.PASSWORD_EMPTY);
        } else {
            problems.addAll(passwordProblems(passwordValue));
        }

        if (confirmationValue.isEmpty()) {
            problems.add(Problem.CONFIRMATION_EMPTY);
        } else if (!confirmationValue.equals(passwordValue)) {
            problems.add(Problem.CONFIRMATION_MISMATCH);
        }

        return Set.copyOf(problems);
    }

    public static String normalizeEmail(String email) {
        return rustTrim(email).toLowerCase(Locale.ROOT);
    }

    public static boolean isValidEmailFormat(String email) {

        if (utf8Length(email) > EMAIL_MAX_BYTES
                || email.codePoints().anyMatch(RegistrationValidator::isRustWhitespace)) {
            return false;
        }

        int at = email.indexOf('@');

        if (at < 0 || email.indexOf('@', at + 1) >= 0) {
            return false;
        }

        String local = email.substring(0, at);
        String domain = email.substring(at + 1);

        return isValidLocalPart(local) && isValidDomain(domain);
    }

    static Set<Problem> passwordProblems(String password) {

        Set<Problem> problems = EnumSet.noneOf(Problem.class);
        int length = password.codePointCount(0, password.length());

        if (length < PASSWORD_MIN_LENGTH) {
            problems.add(Problem.PASSWORD_TOO_SHORT);
        } else if (length > PASSWORD_MAX_LENGTH) {
            problems.add(Problem.PASSWORD_TOO_LONG);
        }

        if (password.codePoints().noneMatch(Character::isUpperCase)) {
            problems.add(Problem.PASSWORD_MISSING_UPPERCASE);
        }

        if (password.codePoints().noneMatch(Character::isLowerCase)) {
            problems.add(Problem.PASSWORD_MISSING_LOWERCASE);
        }

        if (password.codePoints().noneMatch(codePoint -> codePoint >= '0' && codePoint <= '9')) {
            problems.add(Problem.PASSWORD_MISSING_DIGIT);
        }

        if (password.codePoints().allMatch(RegistrationValidator::isRustAlphanumeric)) {
            problems.add(Problem.PASSWORD_MISSING_SYMBOL);
        }

        return problems;
    }

    private static boolean isValidLocalPart(String local) {

        if (local.isEmpty()
                || local.length() > LOCAL_MAX_BYTES
                || local.startsWith(".")
                || local.endsWith(".")
                || local.contains("..")) {
            return false;
        }

        return local.chars()
                .allMatch(
                        character ->
                                isAsciiAlphanumeric(character)
                                        || LOCAL_SPECIALS.indexOf(character) >= 0);
    }

    private static boolean isValidDomain(String domain) {

        if (!domain.contains(".")) {
            return false;
        }

        for (String label : domain.split("\\.", -1)) {
            if (label.isEmpty()
                    || label.length() > LABEL_MAX_BYTES
                    || label.startsWith("-")
                    || label.endsWith("-")
                    || !label.chars()
                            .allMatch(
                                    character ->
                                            isAsciiAlphanumeric(character) || character == '-')) {
                return false;
            }
        }
        return true;
    }

    private static boolean isAsciiAlphanumeric(int character) {
        return (character >= 'a' && character <= 'z')
                || (character >= 'A' && character <= 'Z')
                || (character >= '0' && character <= '9');
    }

    static boolean isRustAlphanumeric(int codePoint) {

        if (Character.isAlphabetic(codePoint)) {
            return true;
        }

        int characterType = Character.getType(codePoint);

        return characterType == Character.DECIMAL_DIGIT_NUMBER
                || characterType == Character.LETTER_NUMBER
                || characterType == Character.OTHER_NUMBER;
    }

    // Character.isWhitespace does not implement the same Unicode White_Space set as Rust.
    static boolean isRustWhitespace(int codePoint) {
        return (codePoint >= 0x09 && codePoint <= 0x0D)
                || codePoint == 0x20
                || codePoint == 0x85
                || codePoint == 0xA0
                || codePoint == 0x1680
                || (codePoint >= 0x2000 && codePoint <= 0x200A)
                || codePoint == 0x2028
                || codePoint == 0x2029
                || codePoint == 0x202F
                || codePoint == 0x205F
                || codePoint == 0x3000;
    }

    private static String rustTrim(String text) {
        int start = 0;
        int end = text.length();

        while (start < end && isRustWhitespace(text.codePointAt(start))) {
            start += Character.charCount(text.codePointAt(start));
        }

        while (end > start && isRustWhitespace(text.codePointBefore(end))) {
            end -= Character.charCount(text.codePointBefore(end));
        }

        return text.substring(start, end);
    }

    private static int utf8Length(String text) {
        return text.getBytes(StandardCharsets.UTF_8).length;
    }
}
