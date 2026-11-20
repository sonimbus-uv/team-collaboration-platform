import 'package:flutter/material.dart';

import '../theme/app-theme-extension.dart';
import '../theme/theme-scope.dart';

class RegisterScreen extends StatefulWidget {
  const RegisterScreen({super.key});

  @override
  State<RegisterScreen> createState() => _RegisterScreenState();
}

class _RegisterScreenState extends State<RegisterScreen> {
  final _formKey = GlobalKey<FormState>();
  final _nameController = TextEditingController();
  final _emailController = TextEditingController();
  final _passwordController = TextEditingController();
  final _confirmPasswordController = TextEditingController();

  bool _obscurePassword = true;
  bool _obscureConfirmPassword = true;

  // Estados de error basados en el FXML
  bool _hasGeneralError = false;
  String _generalErrorMessage = '';
  bool _showResendLink = false;
  bool _hasConfirmPasswordError = false;

  @override
  void dispose() {
    _nameController.dispose();
    _emailController.dispose();
    _passwordController.dispose();
    _confirmPasswordController.dispose();
    super.dispose();
  }

  void _togglePasswordVisibility() {
    setState(() => _obscurePassword = !_obscurePassword);
  }

  void _toggleConfirmPasswordVisibility() {
    setState(() => _obscureConfirmPassword = !_obscureConfirmPassword);
  }

  void _handleRegister() {
    if (_passwordController.text != _confirmPasswordController.text) {
      setState(() => _hasConfirmPasswordError = true);
      return;
    } else {
      setState(() => _hasConfirmPasswordError = false);
    }

    // TODO: Conectar con Auth Service (RF-AUTH-01)
    if (_nameController.text.isEmpty ||
        _emailController.text.isEmpty ||
        _passwordController.text.isEmpty) {
      setState(() {
        _hasGeneralError = true;
        _generalErrorMessage = 'Completa todos los campos correctamente.';
        _showResendLink = false;
      });
    } else {
      setState(() => _hasGeneralError = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final appColors = theme.extension<AppThemeColors>()!;
    final isDark = theme.brightness == Brightness.dark;

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
                      key: _formKey,
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          // Logo
                          Align(
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
                          ),
                          const SizedBox(height: 24),

                          Text(
                            'Crear cuenta',
                            textAlign: TextAlign.center,
                            style: TextStyle(
                              fontSize: 26,
                              fontWeight: FontWeight.w800,
                              color: colorScheme.onSurface,
                            ),
                          ),
                          const SizedBox(height: 8),
                          Text(
                            'Todos los campos son obligatorios',
                            textAlign: TextAlign.center,
                            style: TextStyle(
                              fontSize: 15,
                              color: colorScheme.onSurfaceVariant,
                            ),
                          ),
                          const SizedBox(height: 32),

                          if (_hasGeneralError)
                            _buildGeneralErrorBanner(colorScheme, appColors),

                          Text(
                            'Nombre completo',
                            style: TextStyle(
                              fontWeight: FontWeight.w700,
                              fontSize: 13,
                              color: colorScheme.onSurface,
                            ),
                          ),
                          const SizedBox(height: 8),
                          TextFormField(
                            controller: _nameController,
                            keyboardType: TextInputType.name,
                            cursorColor: colorScheme.primary,
                            style: TextStyle(color: colorScheme.onSurface),
                            decoration: _buildInputDecoration('Tu nombre'),
                          ),
                          const SizedBox(height: 20),

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
                            controller: _emailController,
                            keyboardType: TextInputType.emailAddress,
                            cursorColor: colorScheme.primary,
                            style: TextStyle(color: colorScheme.onSurface),
                            decoration: _buildInputDecoration(
                              'nombre@correo.com',
                            ),
                          ),
                          const SizedBox(height: 20),

                          Text(
                            'Contraseña',
                            style: TextStyle(
                              fontWeight: FontWeight.w700,
                              fontSize: 13,
                              color: colorScheme.onSurface,
                            ),
                          ),
                          const SizedBox(height: 8),
                          TextFormField(
                            controller: _passwordController,
                            obscureText: _obscurePassword,
                            cursorColor: colorScheme.primary,
                            style: TextStyle(color: colorScheme.onSurface),
                            decoration: _buildInputDecoration('••••••••')
                                .copyWith(
                                  suffixIcon: _buildVisibilityToggle(
                                    _obscurePassword,
                                    _togglePasswordVisibility,
                                    colorScheme,
                                    appColors,
                                  ),
                                ),
                          ),
                          const SizedBox(height: 6),
                          Text(
                            'De 8 a 128 caracteres, con al menos una mayúscula, una minúscula, un número y un símbolo.',
                            style: TextStyle(
                              color: colorScheme.onSurfaceVariant,
                              fontSize: 12,
                              height: 1.3,
                            ),
                          ),
                          const SizedBox(height: 20),

                          Text(
                            'Confirmar contraseña',
                            style: TextStyle(
                              fontWeight: FontWeight.w700,
                              fontSize: 13,
                              color: colorScheme.onSurface,
                            ),
                          ),
                          const SizedBox(height: 8),
                          TextFormField(
                            controller: _confirmPasswordController,
                            obscureText: _obscureConfirmPassword,
                            cursorColor: colorScheme.primary,
                            style: TextStyle(color: colorScheme.onSurface),
                            decoration: _buildInputDecoration('••••••••')
                                .copyWith(
                                  suffixIcon: _buildVisibilityToggle(
                                    _obscureConfirmPassword,
                                    _toggleConfirmPasswordVisibility,
                                    colorScheme,
                                    appColors,
                                  ),
                                ),
                          ),

                          if (_hasConfirmPasswordError)
                            Padding(
                              padding: const EdgeInsets.only(top: 6.0),
                              child: Row(
                                children: [
                                  Icon(
                                    Icons.error_outline,
                                    color: colorScheme.error,
                                    size: 16,
                                  ),
                                  const SizedBox(width: 6),
                                  Expanded(
                                    child: Text(
                                      'Las contraseñas no coinciden.',
                                      style: TextStyle(
                                        color: colorScheme.error,
                                        fontSize: 12,
                                        fontWeight: FontWeight.w500,
                                      ),
                                    ),
                                  ),
                                ],
                              ),
                            ),
                          const SizedBox(height: 32),

                          ElevatedButton(
                            style: ElevatedButton.styleFrom(
                              backgroundColor: colorScheme.primary,
                              foregroundColor: colorScheme.onPrimary,
                              padding: const EdgeInsets.symmetric(vertical: 16),
                              elevation: 0,
                              shape: RoundedRectangleBorder(
                                borderRadius: BorderRadius.circular(6),
                              ),
                            ),
                            onPressed: _handleRegister,
                            child: const Text(
                              'Crear cuenta',
                              style: TextStyle(
                                fontSize: 15,
                                fontWeight: FontWeight.bold,
                              ),
                            ),
                          ),
                          const SizedBox(height: 24),

                          Row(
                            children: [
                              Expanded(
                                child: Divider(color: appColors.divider),
                              ),
                              Padding(
                                padding: const EdgeInsets.symmetric(
                                  horizontal: 16,
                                ),
                                child: Text(
                                  'o',
                                  style: TextStyle(
                                    color: colorScheme.onSurfaceVariant,
                                    fontSize: 13,
                                  ),
                                ),
                              ),
                              Expanded(
                                child: Divider(color: appColors.divider),
                              ),
                            ],
                          ),
                          const SizedBox(height: 24),

                          OutlinedButton.icon(
                            style: OutlinedButton.styleFrom(
                              padding: const EdgeInsets.symmetric(vertical: 14),
                              side: BorderSide(color: colorScheme.outline),
                              shape: RoundedRectangleBorder(
                                borderRadius: BorderRadius.circular(6),
                              ),
                              foregroundColor: colorScheme.onSurface,
                            ),
                            onPressed: () {},
                            icon: Container(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 6,
                                vertical: 2,
                              ),
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
                              style: TextStyle(
                                fontSize: 14,
                                fontWeight: FontWeight.w700,
                              ),
                            ),
                          ),
                          const SizedBox(height: 32),

                          Row(
                            mainAxisAlignment: MainAxisAlignment.center,
                            children: [
                              Text(
                                '¿Ya tienes cuenta? ',
                                style: TextStyle(
                                  color: colorScheme.onSurfaceVariant,
                                  fontSize: 14,
                                ),
                              ),
                              GestureDetector(
                                onTap: () {
                                  Navigator.pop(context); // Regresa al Login
                                },
                                child: Text(
                                  'Iniciar sesión',
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
                          ),
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
  }

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

  Widget _buildGeneralErrorBanner(
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    return Container(
      padding: const EdgeInsets.all(12),
      margin: const EdgeInsets.only(bottom: 24.0),
      decoration: BoxDecoration(
        color: appColors.errorBackground,
        border: Border.all(color: colorScheme.error),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(Icons.error_outline, color: colorScheme.error, size: 20),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  _generalErrorMessage,
                  style: TextStyle(
                    color: colorScheme.error,
                    fontSize: 14,
                    fontWeight: FontWeight.w500,
                  ),
                ),
                if (_showResendLink)
                  Padding(
                    padding: const EdgeInsets.only(top: 4.0),
                    child: GestureDetector(
                      onTap: () {},
                      child: Text(
                        '¿No recibiste el correo de verificación?',
                        style: TextStyle(
                          color: colorScheme.primary,
                          fontWeight: FontWeight.w600,
                          decoration: TextDecoration.underline,
                          decorationColor: colorScheme.primary,
                          fontSize: 13,
                        ),
                      ),
                    ),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  InputDecoration _buildInputDecoration(String hint) {
    return InputDecoration(
      hintText: hint,
      contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 16),
    );
  }

  Widget _buildVisibilityToggle(
    bool isObscured,
    VoidCallback onPressed,
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    return Container(
      margin: const EdgeInsets.only(right: 8, top: 4, bottom: 4),
      decoration: BoxDecoration(
        border: Border(left: BorderSide(color: appColors.divider)),
      ),
      child: TextButton(
        onPressed: onPressed,
        style: TextButton.styleFrom(
          foregroundColor: colorScheme.onSurface,
          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(4)),
        ),
        child: Text(
          isObscured ? 'Mostrar' : 'Ocultar',
          style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 13),
        ),
      ),
    );
  }
}
