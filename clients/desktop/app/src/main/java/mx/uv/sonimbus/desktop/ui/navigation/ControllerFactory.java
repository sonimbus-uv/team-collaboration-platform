package mx.uv.sonimbus.desktop.ui.navigation;

import java.lang.reflect.Constructor;
import java.lang.reflect.InvocationTargetException;
import java.util.HashMap;
import java.util.Map;
import java.util.Objects;
import javafx.util.Callback;

/**
 * Crea los controladores de los FXML ({@code FXMLLoader.setControllerFactory}) y les inyecta
 * por constructor los servicios registrados, buscándolos por su tipo exacto.
 *
 * <p>Cada controlador debe tener un solo constructor público. Los esqueletos sin dependencias
 * conservan el constructor vacío implícito y no necesitan cambios.
 */
public final class ControllerFactory implements Callback<Class<?>, Object> {

    private final Map<Class<?>, Object> services = new HashMap<>();

    public <T> ControllerFactory register(Class<T> type, T service) {
        services.put(Objects.requireNonNull(type), Objects.requireNonNull(service));
        return this;
    }

    @Override
    public Object call(Class<?> type) {
        Constructor<?> constructor = soleConstructor(type);
        Class<?>[] parameterTypes = constructor.getParameterTypes();
        Object[] arguments = new Object[parameterTypes.length];
        for (int i = 0; i < parameterTypes.length; i++) {
            arguments[i] = services.get(parameterTypes[i]);
            if (arguments[i] == null) {
                throw new IllegalStateException(type.getSimpleName() + " necesita "
                        + parameterTypes[i].getSimpleName() + ", que no está registrado");
            }
        }
        try {
            return constructor.newInstance(arguments);
        } catch (InvocationTargetException e) {
            throw new IllegalStateException("No se pudo crear " + type.getSimpleName(), e.getCause());
        } catch (ReflectiveOperationException e) {
            throw new IllegalStateException("No se pudo crear " + type.getSimpleName(), e);
        }
    }

    private static Constructor<?> soleConstructor(Class<?> type) {
        Constructor<?>[] constructors = type.getConstructors();
        if (constructors.length != 1) {
            throw new IllegalStateException(type.getSimpleName()
                    + " debe tener exactamente un constructor público; tiene " + constructors.length);
        }
        return constructors[0];
    }
}
