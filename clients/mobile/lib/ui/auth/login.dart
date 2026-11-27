import 'package:flutter/material.dart';

import '../../controllers/auth/loginController.dart';
import '../theme/app-theme-extension.dart';
import '../theme/theme-scope.dart';

class LoginScreen extends StatefulWidget {
  const LoginScreen({super.key});

  @override
  State<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends State<LoginScreen> {
  late final LoginController _controller;

  @override
  void initState() {
    super.initState();
    _controller = LoginController();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final appColors = theme.extension<AppThemeColors>()!;
    final isDark = theme.brightness == Brightness.dark;

    return ListenableBuilder(
      listenable: _controller,
      builder: (context, child) {
        return Scaffold(
          body: SafeArea(
            child: Center(
              child: SingleChildScrollView(
                padding: const EdgeInsets.all(24.0),
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 420),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Align(
                        alignment: Alignment.centerRight,
                        child: _buildThemeToggle(context, colorScheme, isDark),
                      ),
                      const SizedBox(height: 12),
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 32.0,
                          vertical: 40.0,
                        ),
                        decoration: BoxDecoration(
                          color: colorScheme.surface,
                          borderRadius: BorderRadius.circular(12),
                          border: Border.all(color: appColors.divider),
                          boxShadow: isDark
                              ? null
                              : [
                                  BoxShadow(
                                    color: colorScheme.onSurface.withValues(
                                      alpha: 0.06,
                                    ),
                                    blurRadius: 15,
                                    offset: const Offset(0, 5),
                                  ),
                                ],
                        ),
                        child: Form(
                          key: _controller.formKey,
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.stretch,
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              _buildLogo(colorScheme),
                              const SizedBox(height: 24),
                              Text(
                                'Iniciar sesión',
                                textAlign: TextAlign.center,
                                style: TextStyle(
                                  fontSize: 26,
                                  fontWeight: FontWeight.w800,
                                  color: colorScheme.onSurface,
                                ),
                              ),
                              const SizedBox(height: 8),
                              Text(
                                'Entra a tus grupos de Sonimbus',
                                textAlign: TextAlign.center,
                                style: TextStyle(
                                  fontSize: 15,
                                  color: colorScheme.onSurfaceVariant,
                                ),
                              ),
                              const SizedBox(height: 32),

                              if (_controller.hasError)
                                _buildErrorBanner(colorScheme, appColors),

                              Text(
                                'Correo electrónico',
                                style: TextStyle(
                                  fontWeight: FontWeight.w700,
                                  fontSize: 13,
                                  color: colorScheme.onSurface,
                                ),
                              ),
                              const SizedBox(height: 8),
                              TextFormField(
                                controller: _controller.emailController,
                                keyboardType: TextInputType.emailAddress,
                                cursorColor: colorScheme.primary,
                                enabled: !_controller.isLoading,
                                style: TextStyle(color: colorScheme.onSurface),
                                decoration: _inputDecoration(
                                  'nombre@correo.com',
                                ),
                              ),
                              const SizedBox(height: 20),

                              Row(
                                mainAxisAlignment:
                                    MainAxisAlignment.spaceBetween,
                                children: [
                                  Text(
                                    'Contraseña',
                                    style: TextStyle(
                                      fontWeight: FontWeight.w700,
                                      fontSize: 13,
                                      color: colorScheme.onSurface,
                                    ),
                                  ),
                                  GestureDetector(
                                    onTap: () => Navigator.pushNamed(
                                      context,
                                      '/recover-password',
                                    ),
                                    child: Text(
                                      '¿Olvidaste tu contraseña?',
                                      style: TextStyle(
                                        color: colorScheme.primary,
                                        fontWeight: FontWeight.w600,
                                        decoration: TextDecoration.underline,
                                        decorationColor: colorScheme.primary,
                                        fontSize: 13,
                                      ),
                                    ),
                                  ),
                                ],
                              ),
                              const SizedBox(height: 8),
                              TextFormField(
                                controller: _controller.passwordController,
                                obscureText: _controller.obscurePassword,
                                cursorColor: colorScheme.primary,
                                enabled: !_controller.isLoading,
                                style: TextStyle(color: colorScheme.onSurface),
                                decoration: _inputDecoration('••••••••')
                                    .copyWith(
                                      suffixIcon: _buildVisibilityToggle(
                                        colorScheme,
                                        appColors,
                                      ),
                                    ),
                              ),
                              const SizedBox(height: 24),

                              ElevatedButton(
                                style: ElevatedButton.styleFrom(
                                  backgroundColor: colorScheme.primary,
                                  foregroundColor: colorScheme.onPrimary,
                                  padding: const EdgeInsets.symmetric(
                                    vertical: 16,
                                  ),
                                  elevation: 0,
                                  shape: RoundedRectangleBorder(
                                    borderRadius: BorderRadius.circular(6),
                                  ),
                                ),
                                onPressed: _controller.isLoading
                                    ? null
                                    : () async {
                                        final success = await _controller
                                            .handleLogin();
                                        if (success && context.mounted) {
                                          Navigator.pushReplacementNamed(
                                            context,
                                            '/main-menu',
                                          );
                                        }
                                      },
                                child: _controller.isLoading
                                    ? SizedBox(
                                        height: 20,
                                        width: 20,
                                        child: CircularProgressIndicator(
                                          color: colorScheme.onPrimary,
                                          strokeWidth: 2,
                                        ),
                                      )
                                    : const Text(
                                        'Iniciar sesión',
                                        style: TextStyle(
                                          fontSize: 15,
                                          fontWeight: FontWeight.bold,
                                        ),
                                      ),
                              ),
                              const SizedBox(height: 24),

                              _buildDivider(colorScheme, appColors),
                              const SizedBox(height: 24),
                              _buildGoogleButton(colorScheme, appColors),
                              const SizedBox(height: 32),
                              _buildRegisterLink(colorScheme),
                            ],
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        );
      },
    );
  }

  // --- Widgets visuales extraídos ---

  Widget _buildThemeToggle(
    BuildContext context,
    ColorScheme colorScheme,
    bool isDark,
  ) {
    final themeController = ThemeScope.of(context);
    return OutlinedButton.icon(
      onPressed: themeController.toggleTheme,
      style: OutlinedButton.styleFrom(
        foregroundColor: colorScheme.onSurface,
        backgroundColor: colorScheme.surface,
        side: BorderSide(color: colorScheme.outline),
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
      ),
      icon: Icon(
        isDark ? Icons.light_mode_outlined : Icons.dark_mode_outlined,
        size: 18,
        color: colorScheme.primary,
      ),
      label: Text(
        isDark ? 'Modo claro' : 'Modo oscuro',
        style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
      ),
    );
  }

  Widget _buildLogo(ColorScheme colorScheme) {
    return Align(
      alignment: Alignment.center,
      child: Container(
        width: 64,
        height: 64,
        decoration: BoxDecoration(
          color: colorScheme.primary,
          borderRadius: BorderRadius.circular(12),
        ),
        alignment: Alignment.center,
        child: Text(
          '[Logo]',
          style: TextStyle(
            color: colorScheme.onPrimary,
            fontWeight: FontWeight.bold,
            fontSize: 16,
          ),
        ),
      ),
    );
  }

  Widget _buildErrorBanner(ColorScheme colorScheme, AppThemeColors appColors) {
    return Container(
      padding: const EdgeInsets.all(12),
      margin: const EdgeInsets.only(bottom: 24.0),
      decoration: BoxDecoration(
        color: appColors.errorBackground,
        border: Border.all(color: colorScheme.error),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        children: [
          Icon(Icons.error_outline, color: colorScheme.error, size: 20),
          const SizedBox(width: 10),
          Expanded(
            child: Text(
              _controller.errorMessage,
              style: TextStyle(
                color: colorScheme.error,
                fontSize: 14,
                fontWeight: FontWeight.w500,
              ),
            ),
          ),
        ],
      ),
    );
  }

  InputDecoration _inputDecoration(String hint) {
    return InputDecoration(
      hintText: hint,
      contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 16),
    );
  }

  Widget _buildVisibilityToggle(
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    return Container(
      margin: const EdgeInsets.only(right: 8, top: 4, bottom: 4),
      decoration: BoxDecoration(
        border: Border(left: BorderSide(color: appColors.divider)),
      ),
      child: TextButton(
        onPressed: _controller.togglePasswordVisibility,
        style: TextButton.styleFrom(
          foregroundColor: colorScheme.onSurface,
          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(4)),
        ),
        child: Text(
          _controller.obscurePassword ? 'Mostrar' : 'Ocultar',
          style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 13),
        ),
      ),
    );
  }

  Widget _buildDivider(ColorScheme colorScheme, AppThemeColors appColors) {
    return Row(
      children: [
        Expanded(child: Divider(color: appColors.divider)),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: Text(
            'o',
            style: TextStyle(color: colorScheme.onSurfaceVariant, fontSize: 13),
          ),
        ),
        Expanded(child: Divider(color: appColors.divider)),
      ],
    );
  }

  Widget _buildGoogleButton(ColorScheme colorScheme, AppThemeColors appColors) {
    return OutlinedButton.icon(
      style: OutlinedButton.styleFrom(
        padding: const EdgeInsets.symmetric(vertical: 14),
        side: BorderSide(color: colorScheme.outline),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
        foregroundColor: colorScheme.onSurface,
      ),
      onPressed: () {},
      icon: Container(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
        decoration: BoxDecoration(
          border: Border.all(color: colorScheme.outline),
          borderRadius: BorderRadius.circular(4),
          color: appColors.highlight,
        ),
        child: Text(
          'G',
          style: TextStyle(
            fontWeight: FontWeight.bold,
            fontSize: 13,
            color: colorScheme.primary,
          ),
        ),
      ),
      label: const Text(
        'Continuar con Google',
        style: TextStyle(fontSize: 14, fontWeight: FontWeight.w700),
      ),
    );
  }

  Widget _buildRegisterLink(ColorScheme colorScheme) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        Text(
          '¿No tienes cuenta? ',
          style: TextStyle(color: colorScheme.onSurfaceVariant, fontSize: 14),
        ),
        GestureDetector(
          onTap: () => Navigator.pushNamed(context, '/register'),
          child: Text(
            'Crear cuenta',
            style: TextStyle(
              color: colorScheme.primary,
              fontWeight: FontWeight.w800,
              decoration: TextDecoration.underline,
              decorationColor: colorScheme.primary,
              fontSize: 14,
            ),
          ),
        ),
      ],
    );
  }
}
