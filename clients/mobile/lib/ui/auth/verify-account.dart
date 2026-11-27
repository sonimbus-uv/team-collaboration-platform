import 'package:flutter/material.dart';

import '../theme/app-theme-extension.dart';
import '../theme/theme-scope.dart';

class VerifyAccountScreen extends StatefulWidget {
  const VerifyAccountScreen({super.key});

  @override
  State<VerifyAccountScreen> createState() => _VerifyAccountScreenState();
}

class _VerifyAccountScreenState extends State<VerifyAccountScreen> {
  // Simulamos el correo que se pasaría como argumento tras el registro
  final String _sentToEmail = 'tu correo';

  bool _hasError = false;
  final String _errorMessage = 'Ocurrió un error al enviar el correo.';

  bool _hasInfo = false;
  final String _infoMessage =
      'Si la cuenta existe y no está verificada, te enviamos un nuevo enlace.';

  void _handleAlreadyVerified() {
    // TODO: Redirigir al Login para que el usuario inicie sesión ya verificado
    Navigator.pop(context);
  }

  void _handleResendEmail() {
    // TODO: Conectar con Auth Service para reenviar el correo
    setState(() {
      _hasError = false;
      _hasInfo = true;
    });
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
                        Align(
                          alignment: Alignment.center,
                          child: Container(
                            width: 64,
                            height: 64,
                            decoration: BoxDecoration(
                              color: appColors.selectedItem,
                              borderRadius: BorderRadius.circular(12),
                              border: Border.all(color: colorScheme.primary),
                            ),
                            alignment: Alignment.center,
                            child: Icon(
                              Icons.mark_email_unread_outlined,
                              color: colorScheme.primary,
                              size: 32,
                            ),
                          ),
                        ),
                        const SizedBox(height: 24),

                        Text(
                          'Verifica tu cuenta',
                          textAlign: TextAlign.center,
                          style: TextStyle(
                            fontSize: 26,
                            fontWeight: FontWeight.w800,
                            color: colorScheme.onSurface,
                          ),
                        ),
                        const SizedBox(height: 12),

                        RichText(
                          textAlign: TextAlign.center,
                          text: TextSpan(
                            style: TextStyle(
                              fontSize: 15,
                              color: colorScheme.onSurfaceVariant,
                              height: 1.4,
                            ),
                            children: [
                              const TextSpan(
                                text: 'Enviamos un enlace de verificación a\n',
                              ),
                              TextSpan(
                                text: _sentToEmail,
                                style: TextStyle(
                                  fontWeight: FontWeight.bold,
                                  color: colorScheme.onSurface,
                                ),
                              ),
                              const TextSpan(
                                text: '.\nÁbrelo desde tu correo para activar tu cuenta.',
                              ),
                            ],
                          ),
                        ),
                        const SizedBox(height: 32),

                        if (_hasError)
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
                                Icon(
                                  Icons.error_outline,
                                  color: colorScheme.error,
                                  size: 20,
                                ),
                                const SizedBox(width: 10),
                                Expanded(
                                  child: Text(
                                    _errorMessage,
                                    style: TextStyle(
                                      color: colorScheme.error,
                                      fontSize: 14,
                                      fontWeight: FontWeight.w500,
                                    ),
                                  ),
                                ),
                              ],
                            ),
                          ),

                        if (_hasInfo)
                          Container(
                            padding: const EdgeInsets.all(12),
                            margin: const EdgeInsets.only(bottom: 24.0),
                            decoration: BoxDecoration(
                              color: appColors.mintBackground,
                              border: Border.all(
                                color: appColors.mintForeground,
                              ),
                              borderRadius: BorderRadius.circular(8),
                            ),
                            child: Row(
                              children: [
                                Icon(
                                  Icons.check_circle_outline,
                                  color: appColors.mintForeground,
                                  size: 20,
                                ),
                                const SizedBox(width: 10),
                                Expanded(
                                  child: Text(
                                    _infoMessage,
                                    style: TextStyle(
                                      color: appColors.mintForeground,
                                      fontSize: 14,
                                      fontWeight: FontWeight.w600,
                                    ),
                                  ),
                                ),
                              ],
                            ),
                          ),

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
                          onPressed: _handleAlreadyVerified,
                          child: const Text(
                            'Ya verifiqué mi cuenta',
                            style: TextStyle(
                              fontSize: 15,
                              fontWeight: FontWeight.bold,
                            ),
                          ),
                        ),
                        const SizedBox(height: 24),

                        Divider(color: appColors.divider),
                        const SizedBox(height: 24),

                        Text(
                          '¿No te llegó el correo? Revisa tu carpeta de spam.',
                          textAlign: TextAlign.center,
                          style: TextStyle(
                            color: colorScheme.onSurfaceVariant,
                            fontSize: 14,
                          ),
                        ),
                        const SizedBox(height: 12),
                        OutlinedButton(
                          style: OutlinedButton.styleFrom(
                            padding: const EdgeInsets.symmetric(vertical: 14),
                            side: BorderSide(color: colorScheme.outline),
                            shape: RoundedRectangleBorder(
                              borderRadius: BorderRadius.circular(6),
                            ),
                            foregroundColor: colorScheme.onSurface,
                          ),
                          onPressed: _handleResendEmail,
                          child: const Text(
                            'Reenviar correo',
                            style: TextStyle(
                              fontSize: 14,
                              fontWeight: FontWeight.w700,
                            ),
                          ),
                        ),
                        const SizedBox(height: 32),

                        Align(
                          alignment: Alignment.center,
                          child: GestureDetector(
                            onTap: () {
                              Navigator.pop(context);
                            },
                            child: Text(
                              'Volver a iniciar sesión',
                              style: TextStyle(
                                color: colorScheme.primary,
                                fontWeight: FontWeight.w800,
                                decoration: TextDecoration.underline,
                                decorationColor: colorScheme.primary,
                                fontSize: 14,
                              ),
                            ),
                          ),
                        ),
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
}
