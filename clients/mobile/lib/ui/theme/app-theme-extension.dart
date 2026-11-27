import 'package:flutter/material.dart';

import 'app-colors.dart';

@immutable
class AppThemeColors extends ThemeExtension<AppThemeColors> {
  const AppThemeColors({
    required this.sidePanel,
    required this.selectedItem,
    required this.highlight,
    required this.divider,
    required this.mintBackground,
    required this.mintForeground,
    required this.errorBackground,
  });

  final Color sidePanel;
  final Color selectedItem;
  final Color highlight;
  final Color divider;
  final Color mintBackground;
  final Color mintForeground;
  final Color errorBackground;

  static const light = AppThemeColors(
    sidePanel: AppColors.lightSidePanel,
    selectedItem: AppColors.lightSelected,
    highlight: AppColors.lightHighlight,
    divider: AppColors.lightDivider,
    mintBackground: AppColors.lightMintBg,
    mintForeground: AppColors.lightMintFg,
    errorBackground: AppColors.lightErrorBg,
  );

  static const dark = AppThemeColors(
    sidePanel: AppColors.darkSidePanel,
    selectedItem: AppColors.darkSelected,
    highlight: AppColors.darkHighlight,
    divider: AppColors.darkDivider,
    mintBackground: AppColors.darkMintBg,
    mintForeground: AppColors.darkMintFg,
    errorBackground: AppColors.darkErrorBg,
  );

  @override
  AppThemeColors copyWith({
    Color? sidePanel,
    Color? selectedItem,
    Color? highlight,
    Color? divider,
    Color? mintBackground,
    Color? mintForeground,
    Color? errorBackground,
  }) {
    return AppThemeColors(
      sidePanel: sidePanel ?? this.sidePanel,
      selectedItem: selectedItem ?? this.selectedItem,
      highlight: highlight ?? this.highlight,
      divider: divider ?? this.divider,
      mintBackground: mintBackground ?? this.mintBackground,
      mintForeground: mintForeground ?? this.mintForeground,
      errorBackground: errorBackground ?? this.errorBackground,
    );
  }

  @override
  AppThemeColors lerp(ThemeExtension<AppThemeColors>? other, double t) {
    if (other is! AppThemeColors) return this;
    return AppThemeColors(
      sidePanel: Color.lerp(sidePanel, other.sidePanel, t)!,
      selectedItem: Color.lerp(selectedItem, other.selectedItem, t)!,
      highlight: Color.lerp(highlight, other.highlight, t)!,
      divider: Color.lerp(divider, other.divider, t)!,
      mintBackground: Color.lerp(mintBackground, other.mintBackground, t)!,
      mintForeground: Color.lerp(mintForeground, other.mintForeground, t)!,
      errorBackground: Color.lerp(errorBackground, other.errorBackground, t)!,
    );
  }
}
