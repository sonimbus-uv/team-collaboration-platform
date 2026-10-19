package mx.uv.sonimbus.desktop.domain.session;

import java.nio.charset.StandardCharsets;
import java.util.EnumSet;
import java.util.Locale;
import java.util.Set;

/**
 * Validación del formulario de registro (CU-01), idéntica a la de Auth
 * ({@code services/auth/src/validators/auth_validator.rs}) para que el servidor no rechace lo
 * que el cliente aceptó. Si Auth cambia su política (#20), hay que cambiar esta clase y su prueba.
 *
 * <p>Equivalencias con Rust: la longitud de la contraseña se cuenta en puntos de código
 * ({@code chars().count()}); {@code is_uppercase}/{@code is_lowercase} son las propiedades
 * Unicode Uppercase/Lowercase ({@link Character#isUpperCase(int)}); el dígito es ASCII; y
 * {@code is_alphanumeric} es Alphabetic o categoría Nd/Nl/No, más amplio que
 * {@link Character#isLetterOrDigit(int)}.
 */
public final class RegistrationValidator {

    public static final int PASSWORD_MIN_LENGTH = 8;
    public static final int PASSWORD_MAX_LENGTH = 128;

    private static final int EMAIL_MAX_BYTES = 254;
    private static final int LOCAL_MAX_BYTES = 64;
    private static final int LABEL_MAX_BYTES = 63;
    private static final String LOCAL_SPECIALS = ".!#$%&'*+-/=?^_`{|}~";

    /** Problemas del formulario, en el orden en que se muestran. */
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

    /**
     * @param email    correo normalizado como lo guarda Auth (sin espacios a los lados y en
     *                 minúsculas); es el que se envía
     * @param problems vacío si el formulario es válido; ordenado como {@link Problem}
     */
    public record Result(String email, Set<Problem> problems) {
        public boolean isValid() {
            return problems.isEmpty();
        }
    }

    private RegistrationValidator() {
    }

    public static Result validate(String email, String password, String confirmation) {
        String normalized = normalizeEmail(email == null ? "" : email);
        String pass = password == null ? "" : password;
        String confirm = confirmation == null ? "" : confirmation;
        Set<Problem> problems = EnumSet.noneOf(Problem.class);

        if (normalized.isEmpty()) {
            problems.add(Problem.EMAIL_EMPTY);
        } else if (!isValidEmailFormat(normalized)) {
            problems.add(Problem.EMAIL_INVALID);
        }

        if (pass.isEmpty()) {
            problems.add(Problem.PASSWORD_EMPTY);
        } else {
            problems.addAll(passwordProblems(pass));
        }

        if (confirm.isEmpty()) {
            problems.add(Problem.CONFIRMATION_EMPTY);
        } else if (!confirm.equals(pass)) {
            problems.add(Problem.CONFIRMATION_MISMATCH);
        }
        return new Result(normalized, problems);
    }

    /** {@code email.trim().to_lowercase()} de Rust. */
    public static String normalizeEmail(String email) {
        return rustTrim(email).toLowerCase(Locale.ROOT);
    }

    /** {@code is_valid_email_format} de Auth; espera el correo ya normalizado. */
    public static boolean isValidEmailFormat(String email) {
        if (utf8Length(email) > EMAIL_MAX_BYTES || email.codePoints().anyMatch(RegistrationValidator::isRustWhitespace)) {
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

    /**
     * Problemas de longitud y de formato. Auth informa solo el primero (corto, largo o formato);
     * aquí se listan todos los requisitos que faltan para que la ayuda sea concreta.
     */
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
        if (password.codePoints().noneMatch(cp -> cp >= '0' && cp <= '9')) {
            problems.add(Problem.PASSWORD_MISSING_DIGIT);
        }
        if (password.codePoints().allMatch(RegistrationValidator::isRustAlphanumeric)) {
            problems.add(Problem.PASSWORD_MISSING_SYMBOL);
        }
        return problems;
    }

    private static boolean isValidLocalPart(String local) {
        if (local.isEmpty() || local.length() > LOCAL_MAX_BYTES
                || local.startsWith(".") || local.endsWith(".") || local.contains("..")) {
            return false;
        }
        return local.chars().allMatch(c -> isAsciiAlphanumeric(c) || LOCAL_SPECIALS.indexOf(c) >= 0);
    }

    private static boolean isValidDomain(String domain) {
        if (!domain.contains(".")) {
            return false;
        }
        // split con límite -1 para conservar etiquetas vacías, como split('.') de Rust.
        for (String label : domain.split("\\.", -1)) {
            if (label.isEmpty() || label.length() > LABEL_MAX_BYTES
                    || label.startsWith("-") || label.endsWith("-")
                    || !label.chars().allMatch(c -> isAsciiAlphanumeric(c) || c == '-')) {
                return false;
            }
        }
        return true;
    }

    private static boolean isAsciiAlphanumeric(int c) {
        return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9');
    }

    /** {@code char::is_alphanumeric}: Alphabetic, o número de categoría Nd, Nl o No. */
    static boolean isRustAlphanumeric(int cp) {
        if (Character.isAlphabetic(cp)) {
            return true;
        }
        int type = Character.getType(cp);
        return type == Character.DECIMAL_DIGIT_NUMBER
                || type == Character.LETTER_NUMBER
                || type == Character.OTHER_NUMBER;
    }

    /**
     * {@code char::is_whitespace}: propiedad Unicode White_Space. No coincide con
     * {@link Character#isWhitespace(int)}, que excluye los espacios de no separación e incluye
     * U+001C–U+001F.
     */
    static boolean isRustWhitespace(int cp) {
        return (cp >= 0x09 && cp <= 0x0D) || cp == 0x20 || cp == 0x85 || cp == 0xA0 || cp == 0x1680
                || (cp >= 0x2000 && cp <= 0x200A) || cp == 0x2028 || cp == 0x2029 || cp == 0x202F
                || cp == 0x205F || cp == 0x3000;
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
