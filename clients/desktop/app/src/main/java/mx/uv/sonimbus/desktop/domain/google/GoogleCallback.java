package mx.uv.sonimbus.desktop.domain.google;

import java.net.URI;
import java.util.HashMap;
import java.util.Map;
import java.util.UUID;

public final class GoogleCallback {
    private GoogleCallback() {}

    public static Map<String, String> parameters(URI uri) throws GoogleException {
        if (uri == null
                || uri.toString().length() > 2048
                || !"sonimbus".equals(uri.getScheme())
                || !"auth".equals(uri.getRawAuthority())
                || !"/google".equals(uri.getRawPath())
                || uri.getRawFragment() != null
                || uri.getRawQuery() == null) {
            throw new GoogleException(GoogleError.INVALID_RESPONSE);
        }
        Map<String, String> values = new HashMap<>();
        for (String parameter : uri.getRawQuery().split("&", -1)) {
            String[] pair = parameter.split("=", -1);
            if (pair.length != 2 || values.putIfAbsent(pair[0], pair[1]) != null) {
                throw new GoogleException(GoogleError.INVALID_RESPONSE);
            }
        }
        if (!values.keySet().equals(java.util.Set.of("attempt_id", "completion_code"))
                && !values.keySet().equals(java.util.Set.of("attempt_id", "error"))) {
            throw new GoogleException(GoogleError.INVALID_RESPONSE);
        }
        try {
            if (!UUID.fromString(values.get("attempt_id"))
                    .toString()
                    .equals(values.get("attempt_id"))) {
                throw new IllegalArgumentException();
            }
        } catch (RuntimeException malformed) {
            throw new GoogleException(GoogleError.INVALID_RESPONSE);
        }
        String error = values.get("error");
        if (error != null && !error.equals("cancelled") && !error.equals("failed")) {
            throw new GoogleException(GoogleError.INVALID_RESPONSE);
        }
        if (error == null && !values.get("completion_code").matches("[A-Za-z0-9_-]{43}")) {
            throw new GoogleException(GoogleError.INVALID_RESPONSE);
        }
        return Map.copyOf(values);
    }

    public static String completion(URI uri, UUID expectedAttempt) throws GoogleException {
        Map<String, String> values = parameters(uri);
        if (!expectedAttempt.toString().equals(values.get("attempt_id"))) {
            throw new GoogleException(GoogleError.INVALID_RESPONSE);
        }
        if (values.containsKey("error")) {
            throw new GoogleException(
                    values.get("error").equals("cancelled")
                            ? GoogleError.CANCELLED
                            : GoogleError.INVALID_RESPONSE);
        }
        return values.get("completion_code");
    }
}
