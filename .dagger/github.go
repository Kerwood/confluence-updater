package main

import (
	"context"
	"dagger/confluence-updater/internal/dagger"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/google/go-github/v74/github"
)

type Tag struct {
	Name string `json:"name"`
}

type Github struct {
	client *github.Client
	repo   string
	owner  string
}

// ############################################################ //
//                      New Github Client                       //
// ############################################################ //

// Creates a new Github client with no authentication.
//
// Returns *Github struct.
func NewGithubClient() *Github {
	return &Github{
		client: github.NewClient(nil),
		owner:  "kerwood",
		repo:   "confluence-updater",
	}
}

// ############################################################ //
//            Implemented Function: With Auth Token             //
// ############################################################ //

// Adds a *dagger.Secret as authentication to the Github client.
//
// Returns the Github struct. (*Github struct, error)
func (g *Github) WithAuthToken(ctx context.Context, token *dagger.Secret) (*Github, error) {
	plainTextToken, err := token.Plaintext(ctx)
	if err != nil {
		return nil, err
	}
	plainTextToken = strings.TrimSpace(plainTextToken)

	g.client = g.client.WithAuthToken(plainTextToken)
	return g, nil
}

// ############################################################ //
//               Implemented Function: List Tags                //
// ############################################################ //

// List all tags in the repository.
//
// Returns a string slice with all tags. ([]string, error)
func (g *Github) ListTags(ctx context.Context) ([]string, error) {
	var allTags []string

	opts := &github.ListOptions{PerPage: 100}

	for {
		tags, resp, err := g.client.Repositories.ListTags(ctx, g.owner, g.repo, opts)
		if err != nil {
			return nil, fmt.Errorf("Failed to list tags: %v", err)
		}

		for _, tag := range tags {
			allTags = append(allTags, tag.GetName())
		}

		if resp.NextPage == 0 {
			break
		}
		opts.Page = resp.NextPage
	}
	return allTags, nil
}

// ############################################################ //
//             Implemented Function: Create Release             //
// ############################################################ //

// Creates a Github release.
//
// Returns the RepositoryRelease struct. (*github.RepositoryRelease, error)
func (g *Github) CreateRelease(ctx context.Context, tag string) (*github.RepositoryRelease, error) {

	releaseDraft := &github.RepositoryRelease{
		TagName:              github.Ptr(tag),
		TargetCommitish:      github.Ptr("main"),
		Name:                 github.Ptr("confluence-updater-" + tag), //github.Ptr()
		Draft:                github.Ptr(false),
		Prerelease:           github.Ptr(false),
		MakeLatest:           github.Ptr("true"),
		GenerateReleaseNotes: github.Ptr(true),
	}

	release, resp, err := g.client.Repositories.CreateRelease(ctx, g.owner, g.repo, releaseDraft)
	if err != nil {
		return nil, fmt.Errorf("failed to create release: %v. Github response: %v", err, resp)
	}

	return release, nil
}

// ############################################################ //
//              Implemented Function: Upload Asset              //
// ############################################################ //

// Uploads an asset to a given Github release ID.
//
// Returns error if any.
func (g *Github) UploadAsset(ctx context.Context, releaseID int64, filePath string) error {

	file, err := os.Open(filePath)
	if err != nil {
		return fmt.Errorf("unable to read asset file: %v", err)
	}
	defer file.Close()

	_, fileName := filepath.Split(filePath)
	_, resp, err := g.client.Repositories.UploadReleaseAsset(ctx, g.owner, g.repo, releaseID, &github.UploadOptions{Name: fileName}, file)

	if err != nil {
		return fmt.Errorf("failed to upload asset file: %v. Github response: %v", err, resp)
	}

	return nil
}
