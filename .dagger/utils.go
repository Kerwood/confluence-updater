package main

import (
	"context"
	"dagger/confluence-updater/internal/dagger"
	"fmt"

	"github.com/pelletier/go-toml"
)

// Reads the Cargo.toml file and returns the 'version' property as string
func getCargoVersion(ctx context.Context, cargoFilePath *dagger.File) (string, error) {
	content, err := cargoFilePath.Contents(ctx)
	if err != nil {
		return "", fmt.Errorf("failed to read Cargo.toml: %w", err)
	}

	tree, err := toml.Load(content)
	if err != nil {
		return "", fmt.Errorf("failed to parse TOML: %w", err)
	}

	version := tree.Get("package.version")
	if version == nil {
		return "", fmt.Errorf("version not found in Cargo.toml")
	}

	return fmt.Sprintf("%v", version), nil
}
