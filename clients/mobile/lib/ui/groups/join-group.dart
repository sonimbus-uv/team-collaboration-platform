import 'package:flutter/material.dart';

import '../theme/app-theme-extension.dart';

class JoinGroupDialog extends StatefulWidget {
  const JoinGroupDialog({super.key});

  @override
  State<JoinGroupDialog> createState() => _JoinGroupDialogState();
}

class _JoinGroupDialogState extends State<JoinGroupDialog> {
  final _codeController = TextEditingController();

  bool _hasError = false;
  String _errorMessage = '';

  @override
  void dispose() {
    _codeController.dispose();
    super.dispose();
  }

  void _handleJoin() {
    if (_codeController.text.trim().isEmpty) {
      setState(() {
        _hasError = true;
        _errorMessage = 'Ingresa un código de invitación válido.';
      });
      return;
    }

    setState(() => _hasError = false);

    // TODO: Ejecutar lógica del CU-11 / RF-GRP-03 para validar el código en el backend.
    // Si la validación es exitosa, se cierra el diálogo:
    // Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final appColors = theme.extension<AppThemeColors>()!;

    return AlertDialog(
      backgroundColor: colorScheme.surface,
      surfaceTintColor: Colors.transparent,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
        side: BorderSide(color: appColors.divider),
      ),
      contentPadding: const EdgeInsets.all(24),
      titlePadding: const EdgeInsets.only(left: 24, top: 24, right: 24),
      title: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(
            'Unirse a un grupo',
            style: TextStyle(
              fontWeight: FontWeight.bold,
              fontSize: 18,
              color: colorScheme.onSurface,
            ),
          ),
          IconButton(
            tooltip: 'Cerrar',
            icon: Icon(
              Icons.close,
              size: 20,
              color: colorScheme.onSurfaceVariant,
            ),
            padding: EdgeInsets.zero,
            constraints: const BoxConstraints(),
            onPressed: () => Navigator.of(context).pop(),
          ),
        ],
      ),
      content: SizedBox(
        width: MediaQuery.of(context).size.width * 0.9,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              'Escribe el código de invitación que te compartió un administrador del grupo.',
              style: TextStyle(
                color: colorScheme.onSurfaceVariant,
                fontSize: 14,
              ),
            ),
            const SizedBox(height: 16),

            if (_hasError)
              Container(
                padding: const EdgeInsets.all(10),
                margin: const EdgeInsets.only(bottom: 16),
                decoration: BoxDecoration(
                  color: appColors.errorBackground,
                  border: Border.all(color: colorScheme.error),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: Row(
                  children: [
                    Icon(
                      Icons.error_outline,
                      color: colorScheme.error,
                      size: 18,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        _errorMessage,
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

            Text(
              'Código de invitación',
              style: TextStyle(
                fontWeight: FontWeight.w700,
                fontSize: 13,
                color: colorScheme.onSurface,
              ),
            ),
            const SizedBox(height: 6),
            TextFormField(
              controller: _codeController,
              cursorColor: colorScheme.primary,
              style: TextStyle(color: colorScheme.onSurface),
              decoration: const InputDecoration(
                contentPadding: EdgeInsets.symmetric(
                  horizontal: 14,
                  vertical: 12,
                ),
              ),
            ),
            const SizedBox(height: 6),
            Text(
              'También puedes pegar el enlace de invitación completo.',
              style: TextStyle(
                color: colorScheme.onSurfaceVariant,
                fontSize: 12,
              ),
            ),
          ],
        ),
      ),
      actionsPadding: const EdgeInsets.only(right: 24, bottom: 24, left: 24),
      actions: [
        OutlinedButton(
          onPressed: () => Navigator.of(context).pop(),
          style: OutlinedButton.styleFrom(
            foregroundColor: colorScheme.onSurface,
            side: BorderSide(color: colorScheme.outline),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(6),
            ),
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
          ),
          child: const Text(
            'Cancelar',
            style: TextStyle(fontWeight: FontWeight.bold),
          ),
        ),
        ElevatedButton(
          onPressed: _handleJoin,
          style: ElevatedButton.styleFrom(
            backgroundColor: colorScheme.primary,
            foregroundColor: colorScheme.onPrimary,
            elevation: 0,
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(6),
            ),
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
          ),
          child: const Text(
            'Unirse',
            style: TextStyle(fontWeight: FontWeight.bold),
          ),
        ),
      ],
    );
  }
}
