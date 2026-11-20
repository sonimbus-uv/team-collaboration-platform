import 'package:flutter/material.dart';

import '../theme/app-theme-extension.dart';
import '../theme/theme-scope.dart';

class RecoverPasswordScreen extends StatefulWidget {
  const RecoverPasswordScreen({super.key});

  @override
  State<RecoverPasswordScreen> createState() => _RecoverPasswordScreenState();
}

class _RecoverPasswordScreenState extends State<RecoverPasswordScreen> {
  final _formKeyStep1 = GlobalKey<FormState>();
  final _formKeyStep2 = GlobalKey<FormState>();

  final _emailController = TextEditingController();
  final _codeController = TextEditingController();
  final _newPasswordController = TextEditingController();
  final _confirmPasswordField = TextEditingController();

  int _currentStep = 1; // 1 = Correo, 2 = Nueva contraseña

  bool _obscureNewPassword = true;
  bool _obscureConfirmPassword = true;

  bool _showSentInfo = false;
  bool _hasResetError = false;
  bool _hasConfirmPasswordError = false;
  final String _resetErrorMessage =
      'Código de recuperación inválido o caducado.';

  @override
  void dispose() {
    _emailController.dispose();
    _codeController.dispose();
    _newPasswordController.dispose();
    _confirmPasswordField.dispose();
    super.dispose();
  }

  void _handleSendCode() {
    // TODO: Conectar con Auth Service para enviar el correo (RF-AUTH-04)
    if (_emailController.text.isNotEmpty) {
      setState(() {
        _showSentInfo = true;
      });
    }
  }

  void _handleResetPassword() {
    setState(() {
      _hasResetError = false;
      _hasConfirmPasswordError = false;
    });

    if (_newPasswordController.text != _confirmPasswordField.text) {
      setState(() => _hasConfirmPasswordError = true);
      return;
    }

    // TODO: Conectar con Auth Service para validar código y cambiar contraseña
    if (_codeController.text.isEmpty || _newPasswordController.text.isEmpty) {
      setState(() => _hasResetError = true);
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
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        _buildStepIndicator(colorScheme, appColors),
                        const SizedBox(height: 32),
                        if (_currentStep == 1)
                          _buildStep1(colorScheme, appColors)
                        else
                          _buildStep2(colorScheme, appColors),
                      ],
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

  Widget _buildStepIndicator(
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    return Row(
      children: [
        _buildBadge(1, 'Correo', _currentStep >= 1, colorScheme, appColors),
        Expanded(
          child: Container(
            height: 2,
            margin: const EdgeInsets.symmetric(horizontal: 12),
            color: _currentStep == 2 ? colorScheme.primary : appColors.divider,
          ),
        ),
        _buildBadge(
          2,
          'Nueva contraseña',
          _currentStep == 2,
          colorScheme,
          appColors,
        ),
      ],
    );
  }

  Widget _buildBadge(
    int number,
    String label,
    bool isActive,
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    return Row(
      children: [
        Container(
          width: 24,
          height: 24,
          decoration: BoxDecoration(
            color: isActive ? colorScheme.primary : appColors.divider,
            shape: BoxShape.circle,
          ),
          alignment: Alignment.center,
          child: Text(
            number.toString(),
            style: TextStyle(
              color: isActive ? colorScheme.onPrimary : colorScheme.onSurface,
              fontSize: 12,
              fontWeight: FontWeight.bold,
            ),
          ),
        ),
        const SizedBox(width: 8),
        Text(
          label,
          style: TextStyle(
            color: isActive
                ? colorScheme.onSurface
                : colorScheme.onSurfaceVariant,
            fontWeight: isActive ? FontWeight.bold : FontWeight.normal,
            fontSize: 13,
          ),
        ),
      ],
    );
  }

  Widget _buildStep1(ColorScheme colorScheme, AppThemeColors appColors) {
    return Form(
      key: _formKeyStep1,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Recuperar contraseña',
            style: TextStyle(
              fontSize: 24,
              fontWeight: FontWeight.w800,
              color: colorScheme.onSurface,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            'Escribe el correo de tu cuenta y te enviaremos un código para restablecer tu contraseña.',
            style: TextStyle(
              fontSize: 14,
              color: colorScheme.onSurfaceVariant,
              height: 1.4,
            ),
          ),
          const SizedBox(height: 24),

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
            decoration: _buildInputDecoration('nombre@correo.com'),
          ),
          const SizedBox(height: 24),

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
            onPressed: _handleSendCode,
            child: const Text(
              'Enviar código',
              style: TextStyle(fontSize: 15, fontWeight: FontWeight.bold),
            ),
          ),

          if (_showSentInfo)
            Container(
              padding: const EdgeInsets.all(12),
              margin: const EdgeInsets.only(top: 16.0),
              decoration: BoxDecoration(
                color: appColors.mintBackground,
                border: Border.all(color: appColors.mintForeground),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Icon(
                    Icons.check_circle_outline,
                    color: appColors.mintForeground,
                    size: 20,
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Text(
                      'Si el correo está registrado, recibirás un código para restablecer tu contraseña.',
                      style: TextStyle(
                        color: appColors.mintForeground,
                        fontSize: 13,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                ],
              ),
            ),

          const SizedBox(height: 32),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              GestureDetector(
                onTap: () => Navigator.pop(context),
                child: Text(
                  'Volver a iniciar sesión',
                  style: TextStyle(
                    color: colorScheme.onSurfaceVariant,
                    fontSize: 14,
                  ),
                ),
              ),
              GestureDetector(
                onTap: () => setState(() => _currentStep = 2),
                child: Text(
                  'Ya tengo un código',
                  style: TextStyle(
                    color: colorScheme.primary,
                    fontWeight: FontWeight.w800,
                    fontSize: 14,
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildStep2(ColorScheme colorScheme, AppThemeColors appColors) {
    return Form(
      key: _formKeyStep2,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Crea una nueva contraseña',
            style: TextStyle(
              fontSize: 24,
              fontWeight: FontWeight.w800,
              color: colorScheme.onSurface,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            'Escribe el código que llegó a tu correo y elige tu nueva contraseña.',
            style: TextStyle(
              fontSize: 14,
              color: colorScheme.onSurfaceVariant,
              height: 1.4,
            ),
          ),
          const SizedBox(height: 24),

          if (_hasResetError)
            Container(
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
                      _resetErrorMessage,
                      style: TextStyle(
                        color: colorScheme.error,
                        fontSize: 13,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                  ),
                ],
              ),
            ),

          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                'Código de recuperación',
                style: TextStyle(
                  fontWeight: FontWeight.w700,
                  fontSize: 13,
                  color: colorScheme.onSurface,
                ),
              ),
              GestureDetector(
                onTap: () {
                  // TODO: Reenviar código
                },
                child: Text(
                  'Reenviar código',
                  style: TextStyle(
                    color: colorScheme.primary,
                    fontWeight: FontWeight.w600,
                    decoration: TextDecoration.underline,
                    decorationColor: colorScheme.primary,
                    fontSize: 12,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          TextFormField(
            controller: _codeController,
            keyboardType: TextInputType.text,
            cursorColor: colorScheme.primary,
            style: TextStyle(color: colorScheme.onSurface),
            decoration: _buildInputDecoration('000000', letterSpacing: 2),
          ),
          const SizedBox(height: 20),

          Text(
            'Nueva contraseña',
            style: TextStyle(
              fontWeight: FontWeight.w700,
              fontSize: 13,
              color: colorScheme.onSurface,
            ),
          ),
          const SizedBox(height: 8),
          TextFormField(
            controller: _newPasswordController,
            obscureText: _obscureNewPassword,
            cursorColor: colorScheme.primary,
            style: TextStyle(color: colorScheme.onSurface),
            decoration: _buildInputDecoration('••••••••').copyWith(
              suffixIcon: _buildVisibilityToggle(
                _obscureNewPassword,
                () =>
                    setState(() => _obscureNewPassword = !_obscureNewPassword),
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
            'Confirmar nueva contraseña',
            style: TextStyle(
              fontWeight: FontWeight.w700,
              fontSize: 13,
              color: colorScheme.onSurface,
            ),
          ),
          const SizedBox(height: 8),
          TextFormField(
            controller: _confirmPasswordField,
            obscureText: _obscureConfirmPassword,
            cursorColor: colorScheme.primary,
            style: TextStyle(color: colorScheme.onSurface),
            decoration: _buildInputDecoration('••••••••').copyWith(
              suffixIcon: _buildVisibilityToggle(
                _obscureConfirmPassword,
                () => setState(
                  () => _obscureConfirmPassword = !_obscureConfirmPassword,
                ),
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
                  Icon(Icons.error_outline, color: colorScheme.error, size: 16),
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
            onPressed: _handleResetPassword,
            child: const Text(
              'Restablecer contraseña',
              style: TextStyle(fontSize: 15, fontWeight: FontWeight.bold),
            ),
          ),
          const SizedBox(height: 24),

          Align(
            alignment: Alignment.center,
            child: GestureDetector(
              onTap: () => Navigator.pop(context),
              child: Text(
                'Volver a iniciar sesión',
                style: TextStyle(
                  color: colorScheme.onSurfaceVariant,
                  fontSize: 14,
                  decoration: TextDecoration.underline,
                  decorationColor: colorScheme.onSurfaceVariant,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  InputDecoration _buildInputDecoration(String hint, {double? letterSpacing}) {
    return InputDecoration(
      hintText: hint,
      hintStyle: letterSpacing != null
          ? TextStyle(letterSpacing: letterSpacing)
          : null,
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
