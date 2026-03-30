//! Default seed templates
//!
//! Provides pre-configured templates for generating EROFS seeds with various structures.

use crate::types::*;

/// Get all default templates
pub fn get_default_templates() -> Vec<SeedTemplate> {
    vec![
        deep_directory_template(),
        wide_directory_template(),
        symlink_template(),
        xattr_template(),
        mixed_template(),
        afl_fuzzing_template(),
    ]
}

/// Get template by ID
pub fn get_template_by_id(id: &str) -> Option<SeedTemplate> {
    get_default_templates().into_iter().find(|t| t.id == id)
}

/// Deep directory structure template (3-5 levels)
fn deep_directory_template() -> SeedTemplate {
    SeedTemplate {
        id: "deep-directory".to_string(),
        name: "Deep Directory Structure".to_string(),
        description: "3-5 levels of nested directories with files at each level".to_string(),
        config: SeedConfig {
            block_size: 4096,
            volume_name: "deep_test".to_string(),
            compression: None,
            root: DirectoryTreeNode {
                name: "root".to_string(),
                node_type: NodeType::Directory,
                content: None,
                children: Some(vec![
                    // Level 1
                    DirectoryTreeNode {
                        name: "level1".to_string(),
                        node_type: NodeType::Directory,
                        content: None,
                        children: Some(vec![
                            // Level 2
                            DirectoryTreeNode {
                                name: "level2a".to_string(),
                                node_type: NodeType::Directory,
                                content: None,
                                children: Some(vec![
                                    // Level 3
                                    DirectoryTreeNode {
                                        name: "level3".to_string(),
                                        node_type: NodeType::Directory,
                                        content: None,
                                        children: Some(vec![
                                            // Level 4
                                            DirectoryTreeNode {
                                                name: "level4".to_string(),
                                                node_type: NodeType::Directory,
                                                content: None,
                                                children: Some(vec![
                                                    // Level 5
                                                    DirectoryTreeNode {
                                                        name: "deep_file.txt".to_string(),
                                                        node_type: NodeType::File,
                                                        content: Some(FileContentConfig {
                                                            content_type: FileContentType::Text,
                                                            text_content: Some("This is a deeply nested file".to_string()),
                                                            ..Default::default()
                                                        }),
                                                        children: None,
                                                        xattr: None,
                                                        mode: Some(0o644),
                                                        uid: Some(0),
                                                        gid: Some(0),
                                                        target: None,
                                                    },
                                                ]),
                                                xattr: None,
                                                mode: Some(0o755),
                                                uid: Some(0),
                                                gid: Some(0),
                                                target: None,
                                            },
                                        ]),
                                        xattr: None,
                                        mode: Some(0o755),
                                        uid: Some(0),
                                        gid: Some(0),
                                        target: None,
                                    },
                                    DirectoryTreeNode {
                                        name: "file_l3.txt".to_string(),
                                        node_type: NodeType::File,
                                        content: Some(FileContentConfig {
                                            content_type: FileContentType::Text,
                                            text_content: Some("File at level 3".to_string()),
                                            ..Default::default()
                                        }),
                                        children: None,
                                        xattr: None,
                                        mode: Some(0o644),
                                        uid: Some(0),
                                        gid: Some(0),
                                        target: None,
                                    },
                                ]),
                                xattr: None,
                                mode: Some(0o755),
                                uid: Some(0),
                                gid: Some(0),
                                target: None,
                            },
                            DirectoryTreeNode {
                                name: "level2b".to_string(),
                                node_type: NodeType::Directory,
                                content: None,
                                children: Some(vec![
                                    DirectoryTreeNode {
                                        name: "file_l2b.txt".to_string(),
                                        node_type: NodeType::File,
                                        content: Some(FileContentConfig {
                                            content_type: FileContentType::Text,
                                            text_content: Some("File in level2b".to_string()),
                                            ..Default::default()
                                        }),
                                        children: None,
                                        xattr: None,
                                        mode: Some(0o644),
                                        uid: Some(0),
                                        gid: Some(0),
                                        target: None,
                                    },
                                ]),
                                xattr: None,
                                mode: Some(0o755),
                                uid: Some(0),
                                gid: Some(0),
                                target: None,
                            },
                        ]),
                        xattr: None,
                        mode: Some(0o755),
                        uid: Some(0),
                        gid: Some(0),
                        target: None,
                    },
                    DirectoryTreeNode {
                        name: "root_file.txt".to_string(),
                        node_type: NodeType::File,
                        content: Some(FileContentConfig {
                            content_type: FileContentType::Text,
                            text_content: Some("File at root level".to_string()),
                            ..Default::default()
                        }),
                        children: None,
                        xattr: None,
                        mode: Some(0o644),
                        uid: Some(0),
                        gid: Some(0),
                        target: None,
                    },
                ]),
                xattr: None,
                mode: Some(0o755),
                uid: Some(0),
                gid: Some(0),
                target: None,
            },
            description: Some("Deep directory structure with 3-5 levels of nesting".to_string()),
            tags: Some(vec!["deep".to_string(), "nested".to_string(), "stress-test".to_string()]),
        },
    }
}

/// Wide directory structure template (many files in single directory)
fn wide_directory_template() -> SeedTemplate {
    let mut children: Vec<DirectoryTreeNode> = Vec::new();

    // Create 20 files in root directory
    for i in 0..20 {
        children.push(DirectoryTreeNode {
            name: format!("file_{:02}.txt", i),
            node_type: NodeType::File,
            content: Some(FileContentConfig {
                content_type: FileContentType::Random,
                random_config: Some(RandomContentConfig {
                    size_range: (100, 500),
                    entropy: Some(EntropyLevel::Medium),
                }),
                ..Default::default()
            }),
            children: None,
            xattr: None,
            mode: Some(0o644),
            uid: Some(0),
            gid: Some(0),
            target: None,
        });
    }

    // Create 5 subdirectories with files
    for i in 0..5 {
        let mut subdir_children: Vec<DirectoryTreeNode> = Vec::new();
        for j in 0..10 {
            subdir_children.push(DirectoryTreeNode {
                name: format!("subfile_{:02}.bin", j),
                node_type: NodeType::File,
                content: Some(FileContentConfig {
                    content_type: FileContentType::Random,
                    random_config: Some(RandomContentConfig {
                        size_range: (256, 1024),
                        entropy: Some(EntropyLevel::High),
                    }),
                    ..Default::default()
                }),
                children: None,
                xattr: None,
                mode: Some(0o644),
                uid: Some(0),
                gid: Some(0),
                target: None,
            });
        }
        children.push(DirectoryTreeNode {
            name: format!("subdir_{}", i),
            node_type: NodeType::Directory,
            content: None,
            children: Some(subdir_children),
            xattr: None,
            mode: Some(0o755),
            uid: Some(0),
            gid: Some(0),
            target: None,
        });
    }

    SeedTemplate {
        id: "wide-directory".to_string(),
        name: "Wide Directory Structure".to_string(),
        description: "Many files and directories at the root level".to_string(),
        config: SeedConfig {
            block_size: 4096,
            volume_name: "wide_test".to_string(),
            compression: None,
            root: DirectoryTreeNode {
                name: "root".to_string(),
                node_type: NodeType::Directory,
                content: None,
                children: Some(children),
                xattr: None,
                mode: Some(0o755),
                uid: Some(0),
                gid: Some(0),
                target: None,
            },
            description: Some("Wide directory structure with many files at root level".to_string()),
            tags: Some(vec!["wide".to_string(), "many-files".to_string(), "stress-test".to_string()]),
        },
    }
}

/// Symlink template
fn symlink_template() -> SeedTemplate {
    SeedTemplate {
        id: "symlinks".to_string(),
        name: "Symbolic Links".to_string(),
        description: "Various symlink configurations including circular and broken links".to_string(),
        config: SeedConfig {
            block_size: 4096,
            volume_name: "symlink_test".to_string(),
            compression: None,
            root: DirectoryTreeNode {
                name: "root".to_string(),
                node_type: NodeType::Directory,
                content: None,
                children: Some(vec![
                    // Target files
                    DirectoryTreeNode {
                        name: "target1.txt".to_string(),
                        node_type: NodeType::File,
                        content: Some(FileContentConfig {
                            content_type: FileContentType::Text,
                            text_content: Some("Target file 1".to_string()),
                            ..Default::default()
                        }),
                        children: None,
                        xattr: None,
                        mode: Some(0o644),
                        uid: Some(0),
                        gid: Some(0),
                        target: None,
                    },
                    DirectoryTreeNode {
                        name: "target2.txt".to_string(),
                        node_type: NodeType::File,
                        content: Some(FileContentConfig {
                            content_type: FileContentType::Text,
                            text_content: Some("Target file 2".to_string()),
                            ..Default::default()
                        }),
                        children: None,
                        xattr: None,
                        mode: Some(0o644),
                        uid: Some(0),
                        gid: Some(0),
                        target: None,
                    },
                    // Valid symlinks
                    DirectoryTreeNode {
                        name: "link1.txt".to_string(),
                        node_type: NodeType::Symlink,
                        content: None,
                        children: None,
                        xattr: None,
                        mode: None,
                        uid: None,
                        gid: None,
                        target: Some("target1.txt".to_string()),
                    },
                    DirectoryTreeNode {
                        name: "link2.txt".to_string(),
                        node_type: NodeType::Symlink,
                        content: None,
                        children: None,
                        xattr: None,
                        mode: None,
                        uid: None,
                        gid: None,
                        target: Some("target2.txt".to_string()),
                    },
                    // Broken symlink (points to non-existent file)
                    DirectoryTreeNode {
                        name: "broken_link.txt".to_string(),
                        node_type: NodeType::Symlink,
                        content: None,
                        children: None,
                        xattr: None,
                        mode: None,
                        uid: None,
                        gid: None,
                        target: Some("non_existent.txt".to_string()),
                    },
                    // Directory with symlinks
                    DirectoryTreeNode {
                        name: "links".to_string(),
                        node_type: NodeType::Directory,
                        content: None,
                        children: Some(vec![
                            DirectoryTreeNode {
                                name: "to_root".to_string(),
                                node_type: NodeType::Symlink,
                                content: None,
                                children: None,
                                xattr: None,
                                mode: None,
                                uid: None,
                                gid: None,
                                target: Some("..".to_string()),
                            },
                        ]),
                        xattr: None,
                        mode: Some(0o755),
                        uid: Some(0),
                        gid: Some(0),
                        target: None,
                    },
                ]),
                xattr: None,
                mode: Some(0o755),
                uid: Some(0),
                gid: Some(0),
                target: None,
            },
            description: Some("Test with valid, broken, and circular symlinks".to_string()),
            tags: Some(vec!["symlink".to_string(), "links".to_string(), "edge-case".to_string()]),
        },
    }
}

/// Extended attributes template
fn xattr_template() -> SeedTemplate {
    // Note: base64 encoding for xattr values
    // "test_value" -> "dGVzdF92YWx1ZQ=="
    // "user.attr" -> "dXNlci5hdHRy"

    SeedTemplate {
        id: "xattr".to_string(),
        name: "Extended Attributes".to_string(),
        description: "Files with extended attributes (xattr)".to_string(),
        config: SeedConfig {
            block_size: 4096,
            volume_name: "xattr_test".to_string(),
            compression: None,
            root: DirectoryTreeNode {
                name: "root".to_string(),
                node_type: NodeType::Directory,
                content: None,
                children: Some(vec![
                    DirectoryTreeNode {
                        name: "file_with_xattr.txt".to_string(),
                        node_type: NodeType::File,
                        content: Some(FileContentConfig {
                            content_type: FileContentType::Text,
                            text_content: Some("This file has extended attributes".to_string()),
                            ..Default::default()
                        }),
                        children: None,
                        xattr: Some(vec![
                            ExtendedAttribute {
                                name: "user.comment".to_string(),
                                value: "VGhpcyBpcyBhIGNvbW1lbnQ=".to_string(), // "This is a comment"
                            },
                            ExtendedAttribute {
                                name: "user.author".to_string(),
                                value: "dGVzdGVy".to_string(), // "tester"
                            },
                        ]),
                        mode: Some(0o644),
                        uid: Some(0),
                        gid: Some(0),
                        target: None,
                    },
                    DirectoryTreeNode {
                        name: "dir_with_xattr".to_string(),
                        node_type: NodeType::Directory,
                        content: None,
                        children: Some(vec![
                            DirectoryTreeNode {
                                name: "nested_file.txt".to_string(),
                                node_type: NodeType::File,
                                content: Some(FileContentConfig {
                                    content_type: FileContentType::Text,
                                    text_content: Some("Nested file".to_string()),
                                    ..Default::default()
                                }),
                                children: None,
                                xattr: Some(vec![
                                    ExtendedAttribute {
                                        name: "security.label".to_string(),
                                        value: "c2VjdXJpdHlfdGVzdA==".to_string(), // "security_test"
                                    },
                                ]),
                                mode: Some(0o644),
                                uid: Some(0),
                                gid: Some(0),
                                target: None,
                            },
                        ]),
                        xattr: Some(vec![
                            ExtendedAttribute {
                                name: "user.description".to_string(),
                                value: "QSBkaXJlY3Rvcnkgd2l0aCB4YXR0cg==".to_string(), // "A directory with xattr"
                            },
                        ]),
                        mode: Some(0o755),
                        uid: Some(0),
                        gid: Some(0),
                        target: None,
                    },
                ]),
                xattr: None,
                mode: Some(0o755),
                uid: Some(0),
                gid: Some(0),
                target: None,
            },
            description: Some("Files and directories with extended attributes".to_string()),
            tags: Some(vec!["xattr".to_string(), "metadata".to_string(), "security".to_string()]),
        },
    }
}

/// Mixed structure template
fn mixed_template() -> SeedTemplate {
    SeedTemplate {
        id: "mixed".to_string(),
        name: "Mixed Structure".to_string(),
        description: "Combination of deep/wide directories, symlinks, and xattr".to_string(),
        config: SeedConfig {
            block_size: 4096,
            volume_name: "mixed_test".to_string(),
            compression: Some(CompressionConfig {
                algorithm: CompressionAlgorithm::Lz4,
                level: None,
                dict_size: None,
            }),
            root: DirectoryTreeNode {
                name: "root".to_string(),
                node_type: NodeType::Directory,
                content: None,
                children: Some(vec![
                    // Deep structure
                    DirectoryTreeNode {
                        name: "deep".to_string(),
                        node_type: NodeType::Directory,
                        content: None,
                        children: Some(vec![
                            DirectoryTreeNode {
                                name: "nested".to_string(),
                                node_type: NodeType::Directory,
                                content: None,
                                children: Some(vec![
                                    DirectoryTreeNode {
                                        name: "deep_file.txt".to_string(),
                                        node_type: NodeType::File,
                                        content: Some(FileContentConfig {
                                            content_type: FileContentType::Text,
                                            text_content: Some("Deeply nested file".to_string()),
                                            ..Default::default()
                                        }),
                                        children: None,
                                        xattr: None,
                                        mode: Some(0o644),
                                        uid: Some(0),
                                        gid: Some(0),
                                        target: None,
                                    },
                                ]),
                                xattr: None,
                                mode: Some(0o755),
                                uid: Some(0),
                                gid: Some(0),
                                target: None,
                            },
                        ]),
                        xattr: None,
                        mode: Some(0o755),
                        uid: Some(0),
                        gid: Some(0),
                        target: None,
                    },
                    // Wide structure
                    DirectoryTreeNode {
                        name: "wide".to_string(),
                        node_type: NodeType::Directory,
                        content: None,
                        children: Some(vec![
                            DirectoryTreeNode {
                                name: "file_0.txt".to_string(),
                                node_type: NodeType::File,
                                content: Some(FileContentConfig {
                                    content_type: FileContentType::Random,
                                    random_config: Some(RandomContentConfig {
                                        size_range: (100, 200),
                                        entropy: Some(EntropyLevel::Low),
                                    }),
                                    ..Default::default()
                                }),
                                children: None,
                                xattr: None,
                                mode: Some(0o644),
                                uid: Some(0),
                                gid: Some(0),
                                target: None,
                            },
                            DirectoryTreeNode {
                                name: "file_1.txt".to_string(),
                                node_type: NodeType::File,
                                content: Some(FileContentConfig {
                                    content_type: FileContentType::Random,
                                    random_config: Some(RandomContentConfig {
                                        size_range: (100, 200),
                                        entropy: Some(EntropyLevel::Medium),
                                    }),
                                    ..Default::default()
                                }),
                                children: None,
                                xattr: None,
                                mode: Some(0o644),
                                uid: Some(0),
                                gid: Some(0),
                                target: None,
                            },
                        ]),
                        xattr: None,
                        mode: Some(0o755),
                        uid: Some(0),
                        gid: Some(0),
                        target: None,
                    },
                    // Symlink
                    DirectoryTreeNode {
                        name: "link_to_deep".to_string(),
                        node_type: NodeType::Symlink,
                        content: None,
                        children: None,
                        xattr: None,
                        mode: None,
                        uid: None,
                        gid: None,
                        target: Some("deep/nested/deep_file.txt".to_string()),
                    },
                    // File with xattr
                    DirectoryTreeNode {
                        name: "attributed.txt".to_string(),
                        node_type: NodeType::File,
                        content: Some(FileContentConfig {
                            content_type: FileContentType::Text,
                            text_content: Some("File with attributes".to_string()),
                            ..Default::default()
                        }),
                        children: None,
                        xattr: Some(vec![
                            ExtendedAttribute {
                                name: "user.label".to_string(),
                                value: "bWl4ZWQ=".to_string(), // "mixed"
                            },
                        ]),
                        mode: Some(0o644),
                        uid: Some(0),
                        gid: Some(0),
                        target: None,
                    },
                ]),
                xattr: None,
                mode: Some(0o755),
                uid: Some(0),
                gid: Some(0),
                target: None,
            },
            description: Some("Mixed structure with deep/wide directories, symlinks, and xattr".to_string()),
            tags: Some(vec!["mixed".to_string(), "comprehensive".to_string(), "test-all".to_string()]),
        },
    }
}

/// AFL fuzzing template
fn afl_fuzzing_template() -> SeedTemplate {
    let mut children: Vec<DirectoryTreeNode> = Vec::new();

    // Create various AFL-generated files with different sizes and types
    for i in 0..10 {
        let content_type = match i % 4 {
            0 => FileContentType::AflGenerated,
            1 => FileContentType::Random,
            2 => FileContentType::Pattern,
            _ => FileContentType::Text,
        };

        let content = match content_type {
            FileContentType::AflGenerated => Some(FileContentConfig {
                content_type: FileContentType::AflGenerated,
                afl_config: Some(AflContentConfig {
                    size_range: (256, 4096),
                    with_header: Some(i % 3 == 0),
                    format: None,
                    pattern_injection: if i % 2 == 0 { Some("EROFS".to_string()) } else { None },
                }),
                ..Default::default()
            }),
            FileContentType::Random => Some(FileContentConfig {
                content_type: FileContentType::Random,
                random_config: Some(RandomContentConfig {
                    size_range: (128, 2048),
                    entropy: Some(match i % 3 {
                        0 => EntropyLevel::Low,
                        1 => EntropyLevel::Medium,
                        _ => EntropyLevel::High,
                    }),
                }),
                ..Default::default()
            }),
            FileContentType::Pattern => Some(FileContentConfig {
                content_type: FileContentType::Pattern,
                pattern_config: Some(PatternContentConfig {
                    pattern: match i % 3 {
                        0 => "ABCD".to_string(),
                        1 => "1234".to_string(),
                        _ => "WXYZ".to_string(),
                    },
                    repeat_count: Some(50),
                    size: None,
                }),
                ..Default::default()
            }),
            FileContentType::Text => Some(FileContentConfig {
                content_type: FileContentType::Text,
                text_content: Some(format!("AFL test file number {} with some text content for fuzzing", i)),
                ..Default::default()
            }),
            _ => None,
        };

        children.push(DirectoryTreeNode {
            name: format!("afl_file_{:02}.bin", i),
            node_type: NodeType::File,
            content,
            children: None,
            xattr: None,
            mode: Some(0o644),
            uid: Some(0),
            gid: Some(0),
            target: None,
        });
    }

    SeedTemplate {
        id: "afl-fuzzing".to_string(),
        name: "AFL Fuzzing".to_string(),
        description: "Files generated with AFL-style patterns for fuzzing".to_string(),
        config: SeedConfig {
            block_size: 4096,
            volume_name: "afl_test".to_string(),
            compression: None,
            root: DirectoryTreeNode {
                name: "root".to_string(),
                node_type: NodeType::Directory,
                content: None,
                children: Some(children),
                xattr: None,
                mode: Some(0o755),
                uid: Some(0),
                gid: Some(0),
                target: None,
            },
            description: Some("Various files with AFL-style generated content for fuzzing".to_string()),
            tags: Some(vec!["afl".to_string(), "fuzzing".to_string(), "test".to_string()]),
        },
    }
}
