import 'package:flutter/material.dart';

import 'app-colors.dart';
import 'app-theme-extension.dart';

abstract final class AppTheme {
  static ThemeData get light => _buildTheme(
    brightness: Brightness.light,
    background: AppColors.lightBackground,
    surface: AppColors.lightSurface,
    textPrimary: AppColors.lightTextPrimary,
    textSecondary: AppColors.lightTextSecondary,
    primary: AppColors.lightPrimary,
    onPrimary: AppColors.lightOnPrimary,
    controlBorder: AppColors.lightControlBorder,
    divider: AppColors.lightDivider,
    errorFg: AppColors.lightErrorFg,
    errorBg: AppColors.lightErrorBg,
    highlight: AppColors.lightHighlight,
    extension: AppThemeColors.light,
  );

  static ThemeData get dark => _buildTheme(
    brightness: Brightness.dark,
    background: AppColors.darkBackground,
    surface: AppColors.darkSurface,
    textPrimary: AppColors.darkTextPrimary,
    textSecondary: AppColors.darkTextSecondary,
    primary: AppColors.darkPrimary,
    onPrimary: AppColors.darkOnPrimary,
    controlBorder: AppColors.darkControlBorder,
    divider: AppColors.darkDivider,
    errorFg: AppColors.darkErrorFg,
    errorBg: AppColors.darkErrorBg,
    highlight: AppColors.darkHighlight,
    extension: AppThemeColors.dark,
  );

  static ThemeData _buildTheme({
    required Brightness brightness,
    required Color background,
    required Color surface,
    required Color textPrimary,
    required Color textSecondary,
    required Color primary,
    required Color onPrimary,
    required Color controlBorder,
    required Color divider,
    required Color errorFg,
    required Color errorBg,
    required Color highlight,
    required AppThemeColors extension,
  }) {
    final colorScheme = ColorScheme(
      brightness: brightness,
      primary: primary,
      onPrimary: onPrimary,
      secondary: primary,
      onSecondary: onPrimary,
      error: errorFg,
      onError: errorBg,
      surface: surface,
      onSurface: textPrimary,
      onSurfaceVariant: textSecondary,
      outline: controlBorder,
      outlineVariant: divider,
      surfaceContainerHighest: highlight,
    );

    return ThemeData(
      useMaterial3: true,
      brightness: brightness,
      colorScheme: colorScheme,
      scaffoldBackgroundColor: background,
      dividerColor: divider,
      dividerTheme: DividerThemeData(color: divider, thickness: 1),
      cardTheme: CardThemeData(
        color: surface,
        elevation: 0,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12),
          side: BorderSide(color: divider),
        ),
      ),
      dialogTheme: DialogThemeData(
        backgroundColor: surface,
        titleTextStyle: TextStyle(
          color: textPrimary,
          fontSize: 18,
          fontWeight: FontWeight.w600,
        ),
        contentTextStyle: TextStyle(color: textSecondary, fontSize: 14),
      ),
      elevatedButtonTheme: ElevatedButtonThemeData(
        style: ElevatedButton.styleFrom(
          backgroundColor: primary,
          foregroundColor: onPrimary,
          iconColor: onPrimary,
        ),
      ),
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          backgroundColor: primary,
          foregroundColor: onPrimary,
          iconColor: onPrimary,
        ),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: surface,
        labelStyle: TextStyle(color: textSecondary),
        hintStyle: TextStyle(color: textSecondary),
        errorStyle: TextStyle(color: errorFg),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(10),
          borderSide: BorderSide(color: controlBorder),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(10),
          borderSide: BorderSide(color: primary, width: 2),
        ),
        errorBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(10),
          borderSide: BorderSide(color: errorFg),
        ),
        focusedErrorBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(10),
          borderSide: BorderSide(color: errorFg, width: 2),
        ),
      ),
      extensions: <ThemeExtension<dynamic>>[extension],
    );
  }
}
