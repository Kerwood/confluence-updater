// A generated module for ConfluenceUpdater functions
//
// This module has been generated via dagger init and serves as a reference to
// basic module structure as you get started with Dagger.
//
// Two functions have been pre-created. You can modify, delete, or add to them,
// as needed. They demonstrate usage of arguments and return types using simple
// echo and grep commands. The functions can be called from the dagger CLI or
// from one of the SDKs.
//
// The first line in this comment block is a short description line and the
// rest is a long description with more detail on the module's purpose or usage,
// if appropriate. All modules should have a short description.

package main

import (
	"context"
	"dagger/confluence-updater/internal/dagger"
	"fmt"
	"time"
)

type ConfluenceUpdater struct{}

// ############################################################ //
//                      Compile Binary                          //
// ############################################################ //

// Compiles the confluence-updater binary. Returns *dagger.File.
func (m *ConfluenceUpdater) CompileBinary(
	ctx context.Context,
	// +defaultPath="/"
	srcDirectory *dagger.Directory,
) *dagger.File {
	binaryPath := "target/x86_64-unknown-linux-musl/release/confluence-updater"

	container := m.buildEnv().
		WithMountedDirectory("/workdir", srcDirectory).
		WithWorkdir("/workdir").
		WithMountedCache("/workdir/target", dag.CacheVolume("buildCache")).
		WithEnvVariable("OPENSSL_STATIC", "1").
		WithExec([]string{"cargo", "build", "--target", "x86_64-unknown-linux-musl", "--release"}).
		WithExec([]string{"upx", "--best", "--lzma", binaryPath}).
		WithExec([]string{"cp", binaryPath, "/workdir"})

	daggerFile := container.File("/workdir/confluence-updater")

	return daggerFile
}

// ############################################################ //
//                         Build Image                          //
// ############################################################ //

// Creates the build image for compiling the application. Returns *dagger.Container.
func (m *ConfluenceUpdater) buildEnv() *dagger.Container {
	return dag.Container().
		From("rust:1.88-alpine").
		WithExec([]string{"apk", "update"}).
		WithExec([]string{"apk", "add", "musl-dev", "openssl-dev", "openssl-libs-static", "upx"}).
		WithExec([]string{"rustup", "component", "add", "clippy"})
}

// ############################################################ //
//                     Build Runtime Image                      //
// ############################################################ //

// Compiles the binary and builds a runtime time from scratch (blank filesystem). Returns *dagger.Container
func (m *ConfluenceUpdater) BuildRuntimeImage(
	ctx context.Context,
	// +defaultPath="/"
	srcDirectory *dagger.Directory,
) *dagger.Container {

	fromCtr := dag.Container().
		From("alpine:latest").
		WithExec([]string{"apk", "add", "ca-certificates"}).
		WithExec([]string{"update-ca-certificates"}).
		WithExec([]string{"adduser", "--disabled-password", "--gecos", "''", "--home", "/", "--shell", "/sbin/nologin", "--no-create-home", "--uid", "1001", "rust"})

	binary := m.CompileBinary(ctx, srcDirectory)

	container := dag.Container().
		WithLabel("org.opencontainers.image.source", "https://github.com/kerwood/confluence-updater").
		WithLabel("org.opencontainers.image.created", time.Now().String()).
		WithRootfs(dag.Directory()).
		WithFile("/confluence-updater", binary).
		WithFile("/etc/passwd", fromCtr.File("/etc/passwd")).
		WithFile("/etc/group", fromCtr.File("/etc/group")).
		WithDirectory("/etc/ssl/certs", fromCtr.Directory("/etc/ssl/certs")).
		WithUser("1001").
		WithEntrypoint([]string{"/confluence-updater"})

	return container
}

// ############################################################ //
//                             Release                          //
// ############################################################ //

// Checks for version conflicts, builds runtime image, creates Github release, uploads binary as asset and pushes the image.
func (m *ConfluenceUpdater) Release(
	ctx context.Context,
	// +defaultPath="/"
	srcDirectory *dagger.Directory,
	ghUser string,
	ghAuthToken *dagger.Secret,
) error {

	cargoVersion, err := m.CheckVersionConflict(ctx, srcDirectory, ghAuthToken)
	if err != nil {
		return err
	}

	container := m.BuildRuntimeImage(ctx, srcDirectory).
		WithLabel("org.opencontainers.image.version", cargoVersion).
		WithRegistryAuth("ghcr.io", ghUser, ghAuthToken)

	binary := container.File("/confluence-updater")
	binaryExportedPath, err := binary.Export(ctx, "./confluence-updater-x86_64-unknown-linux-musl")

	github, err := NewGithubClient().WithAuthToken(ctx, ghAuthToken)
	if err != nil {
		return err
	}

	release, err := github.CreateRelease(ctx, "v"+cargoVersion)
	if err != nil {
		return err
	}

	err = github.UploadAsset(ctx, *release.ID, binaryExportedPath)
	if err != nil {
		return err
	}

	for _, tag := range []string{"latest", cargoVersion} {
		imageInfo, err := container.Publish(ctx, "ghcr.io/kerwood/confluence-updater:"+tag)
		if err != nil {
			return err
		}
		fmt.Println(imageInfo)
	}

	return nil
}

// ############################################################ //
//                   Check Version Conflict                     //
// ############################################################ //

// Gets the package version from Cargo.toml and validates the tag does not exist in the repo. Returns the Cargo version as string.
func (m *ConfluenceUpdater) CheckVersionConflict(
	ctx context.Context,
	// +defaultPath="/"
	srcDirectory *dagger.Directory,
	ghAuthToken *dagger.Secret,
) (string, error) {

	cargoFile := srcDirectory.File("./Cargo.toml")

	version, err := getCargoVersion(ctx, cargoFile)
	if err != nil {
		return "", err
	}

	github, err := NewGithubClient().WithAuthToken(ctx, ghAuthToken)
	if err != nil {
		return "", err
	}

	tags, err := github.ListTags(ctx)
	if err != nil {
		return "", err
	}

	for _, x := range tags {
		tag := "v" + version
		if x == tag {
			return "", fmt.Errorf("Conflict: Git tag '%s' already exists and matches the declared version in Cargo.toml (%s)", tag, version)
		}
	}

	return version, nil
}

// ############################################################ //
//                             Lint                             //
// ############################################################ //

// Run cargo clippy for linting on code base
func (m *ConfluenceUpdater) Lint(
	// +defaultPath="/"
	srcDirectory *dagger.Directory,
) {
	m.buildEnv().
		WithMountedDirectory("/workdir", srcDirectory).
		WithWorkdir("/workdir").
		WithMountedCache("/workdir/target", dag.CacheVolume("buildCache")).
		WithExec([]string{"cargo", "clippy", "--all-targets", "--all-features", "--", "-D", "warnings"})
}

// ############################################################ //
//                             Fmt                              //
// ############################################################ //

// Run cargo fmt for fomatting checking on code base
func (m *ConfluenceUpdater) Fmt(
	// +defaultPath="/"
	srcDirectory *dagger.Directory,
) {
	m.buildEnv().
		WithMountedDirectory("/workdir", srcDirectory).
		WithWorkdir("/workdir").
		WithExec([]string{"cargo", "fmt", "--check"})
}
