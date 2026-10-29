package mx.uv.sonimbus.desktop.data.api;

/**
 * Par de tokens de {@code login} y {@code refresh}. {@link #toString()} oculta los tokens
 * (RNF-SEG-04).
 *
 * @param accessExpiresIn segundos de vida del access token (máximo 900)
 */
public record Tokens(
        String accessToken,
        String refreshToken,
        String tokenType,
        long accessExpiresIn,
        String sessionExpiresAt,
        String sid,
        String accountId) {

    @Override
    public String toString() {
        return "Tokens[accessToken=[REDACTED], refreshToken=[REDACTED], accessExpiresIn=" + accessExpiresIn
                + ", accountId=" + accountId + "]";
    }
}
