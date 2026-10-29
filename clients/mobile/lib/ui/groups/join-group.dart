import 'package:flutter/material.dart';

class JoinGroupDialog extends StatefulWidget {
  const JoinGroupDialog({super.key});

  @override
  State<JoinGroupDialog> createState() => _JoinGroupDialogState();
}

class _JoinGroupDialogState extends State<JoinGroupDialog> {
  final _codeController = TextEditingController();

  bool _hasError = false;
  String _errorMessage = '';
  final Color _primaryOrange = const Color(0xFFC44111);

  @override
  void dispose() {
    _codeController.dispose();
    super.dispose();
  }

  void _handleJoin() {
    // Validación inicial
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
    return AlertDialog(
      backgroundColor: Colors.white,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
      contentPadding: const EdgeInsets.all(24),
      titlePadding: const EdgeInsets.only(left: 24, top: 24, right: 24),
      title: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          const Text(
            'Unirse a un grupo',
            style: TextStyle(
              fontWeight: FontWeight.bold,
              fontSize: 18,
              color: Colors.black87,
            ),
          ),
          IconButton(
            icon: const Icon(Icons.close, size: 20, color: Colors.black54),
            padding: EdgeInsets.zero,
            constraints: const BoxConstraints(),
            onPressed: () => Navigator.of(context).pop(),
          ),
        ],
      ),
      content: SizedBox(
        width:
            MediaQuery.of(context).size.width *
            0.9, // Ajuste para ocupar ancho en móvil
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const Text(
              'Escribe el código de invitación que te compartió un administrador del grupo.',
              style: TextStyle(color: Colors.black87, fontSize: 14),
            ),
            const SizedBox(height: 16),

            if (_hasError)
              Container(
                padding: const EdgeInsets.all(10),
                margin: const EdgeInsets.only(bottom: 16),
                decoration: BoxDecoration(
                  color: Colors.red.shade50,
                  border: Border.all(color: Colors.red.shade200),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: Row(
                  children: [
                    Icon(
                      Icons.error_outline,
                      color: Colors.red.shade700,
                      size: 18,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        _errorMessage,
                        style: TextStyle(
                          color: Colors.red.shade900,
                          fontSize: 13,
                        ),
                      ),
                    ),
                  ],
                ),
              ),

            const Text(
              'Código de invitación',
              style: TextStyle(
                fontWeight: FontWeight.w700,
                fontSize: 13,
                color: Colors.black87,
              ),
            ),
            const SizedBox(height: 6),
            TextFormField(
              controller: _codeController,
              cursorColor: _primaryOrange,
              decoration: InputDecoration(
                contentPadding: const EdgeInsets.symmetric(
                  horizontal: 14,
                  vertical: 12,
                ),
                border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(6),
                  borderSide: const BorderSide(color: Colors.black26),
                ),
                focusedBorder: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(6),
                  borderSide: BorderSide(color: _primaryOrange, width: 1.5),
                ),
              ),
            ),
            const SizedBox(height: 6),
            const Text(
              'También puedes pegar el enlace de invitación completo.',
              style: TextStyle(color: Colors.black54, fontSize: 12),
            ),
          ],
        ),
      ),
      actionsPadding: const EdgeInsets.only(right: 24, bottom: 24, left: 24),
      actions: [
        OutlinedButton(
          onPressed: () => Navigator.of(context).pop(),
          style: OutlinedButton.styleFrom(
            foregroundColor: Colors.black87,
            side: const BorderSide(color: Colors.black26),
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
            backgroundColor: _primaryOrange,
            foregroundColor: Colors.white,
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
