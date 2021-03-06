// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate url;

use std::char;
use std::net::{Ipv4Addr, Ipv6Addr};
use url::{Host, RelativeSchemeData, SchemeData, Url};


#[test]
fn url_parsing() {
    for test in parse_test_data(include_str!("urltestdata.txt")) {
        let Test {
            input,
            base,
            scheme: expected_scheme,
            username: expected_username,
            password: expected_password,
            host: expected_host,
            port: expected_port,
            path: expected_path,
            query: expected_query,
            fragment: expected_fragment,
            expected_failure,
        } = test;
        let base = match Url::parse(&base) {
            Ok(base) => base,
            Err(message) => panic!("Error parsing base {}: {}", base, message)
        };
        let url = base.join(&input);
        if expected_scheme.is_none() {
            if url.is_ok() && !expected_failure {
                panic!("Expected a parse error for URL {}", input);
            }
            continue
        }
        let Url { scheme, scheme_data, query, fragment, .. } = match url {
            Ok(url) => url,
            Err(message) => {
                if expected_failure {
                    continue
                } else {
                    panic!("Error parsing URL {}: {}", input, message)
                }
            }
        };

        macro_rules! assert_eq {
            ($a: expr, $b: expr) => {
                {
                    let a = $a;
                    let b = $b;
                    if a != b {
                        if expected_failure {
                            continue
                        } else {
                            panic!("{:?} != {:?}", a, b)
                        }
                    }
                }
            }
        }

        assert_eq!(Some(scheme), expected_scheme);
        match scheme_data {
            SchemeData::Relative(RelativeSchemeData {
                username, password, host, port, default_port: _, path,
            }) => {
                assert_eq!(username, expected_username);
                assert_eq!(password, expected_password);
                let host = host.serialize();
                assert_eq!(host, expected_host);
                assert_eq!(port, expected_port);
                assert_eq!(Some(format!("/{}", str_join(&path, "/"))), expected_path);
            },
            SchemeData::NonRelative(scheme_data) => {
                assert_eq!(Some(scheme_data), expected_path);
                assert_eq!(String::new(), expected_username);
                assert_eq!(None, expected_password);
                assert_eq!(String::new(), expected_host);
                assert_eq!(None, expected_port);
            },
        }
        fn opt_prepend(prefix: &str, opt_s: Option<String>) -> Option<String> {
            opt_s.map(|s| format!("{}{}", prefix, s))
        }
        assert_eq!(opt_prepend("?", query), expected_query);
        assert_eq!(opt_prepend("#", fragment), expected_fragment);

        assert!(!expected_failure, "Unexpected success for {}", input);
    }
}

// FIMXE: Remove this when &[&str]::join (the new name) lands in the stable channel.
#[allow(deprecated)]
fn str_join<T: ::std::borrow::Borrow<str>>(pieces: &[T], separator: &str) -> String {
    pieces.connect(separator)
}

struct Test {
    input: String,
    base: String,
    scheme: Option<String>,
    username: String,
    password: Option<String>,
    host: String,
    port: Option<u16>,
    path: Option<String>,
    query: Option<String>,
    fragment: Option<String>,
    expected_failure: bool,
}

fn parse_test_data(input: &str) -> Vec<Test> {
    let mut tests: Vec<Test> = Vec::new();
    for line in input.lines() {
        if line == "" || line.starts_with("#") {
            continue
        }
        let mut pieces = line.split(' ').collect::<Vec<&str>>();
        let expected_failure = pieces[0] == "XFAIL";
        if expected_failure {
            pieces.remove(0);
        }
        let input = unescape(pieces.remove(0));
        let mut test = Test {
            input: input,
            base: if pieces.is_empty() || pieces[0] == "" {
                tests.last().unwrap().base.clone()
            } else {
                unescape(pieces.remove(0))
            },
            scheme: None,
            username: String::new(),
            password: None,
            host: String::new(),
            port: None,
            path: None,
            query: None,
            fragment: None,
            expected_failure: expected_failure,
        };
        for piece in pieces {
            if piece == "" || piece.starts_with("#") {
                continue
            }
            let colon = piece.find(':').unwrap();
            let value = unescape(&piece[colon + 1..]);
            match &piece[..colon] {
                "s" => test.scheme = Some(value),
                "u" => test.username = value,
                "pass" => test.password = Some(value),
                "h" => test.host = value,
                "port" => test.port = Some(value.parse().unwrap()),
                "p" => test.path = Some(value),
                "q" => test.query = Some(value),
                "f" => test.fragment = Some(value),
                _ => panic!("Invalid token")
            }
        }
        tests.push(test)
    }
    tests
}

fn unescape(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars();
    loop {
        match chars.next() {
            None => return output,
            Some(c) => output.push(
                if c == '\\' {
                    match chars.next().unwrap() {
                        '\\' => '\\',
                        'n' => '\n',
                        'r' => '\r',
                        's' => ' ',
                        't' => '\t',
                        'f' => '\x0C',
                        'u' => {
                            char::from_u32((((
                                chars.next().unwrap().to_digit(16).unwrap()) * 16 +
                                chars.next().unwrap().to_digit(16).unwrap()) * 16 +
                                chars.next().unwrap().to_digit(16).unwrap()) * 16 +
                                chars.next().unwrap().to_digit(16).unwrap()).unwrap()
                        }
                        _ => panic!("Invalid test data input"),
                    }
                } else {
                    c
                }
            )
        }
    }
}


#[test]
fn new_file_paths() {
    use std::path::{Path, PathBuf};
    if cfg!(unix) {
        assert_eq!(Url::from_file_path(Path::new("relative")), Err(()));
        assert_eq!(Url::from_file_path(Path::new("../relative")), Err(()));
    } else {
        assert_eq!(Url::from_file_path(Path::new("relative")), Err(()));
        assert_eq!(Url::from_file_path(Path::new(r"..\relative")), Err(()));
        assert_eq!(Url::from_file_path(Path::new(r"\drive-relative")), Err(()));
        assert_eq!(Url::from_file_path(Path::new(r"\\ucn\")), Err(()));
    }

    if cfg!(unix) {
        let mut url = Url::from_file_path(Path::new("/foo/bar")).unwrap();
        assert_eq!(url.host(), Some(&Host::Domain("".to_string())));
        assert_eq!(url.path(), Some(&["foo".to_string(), "bar".to_string()][..]));
        assert!(url.to_file_path() == Ok(PathBuf::from("/foo/bar")));

        url.path_mut().unwrap()[1] = "ba\0r".to_string();
        url.to_file_path().is_ok();

        url.path_mut().unwrap()[1] = "ba%00r".to_string();
        url.to_file_path().is_ok();
    }
}

#[test]
#[cfg(unix)]
fn new_path_bad_utf8() {
    use std::ffi::OsStr;
    use std::os::unix::prelude::*;
    use std::path::{Path, PathBuf};

    let url = Url::from_file_path(Path::new("/foo/ba%80r")).unwrap();
    let os_str = OsStr::from_bytes(b"/foo/ba\x80r");
    assert_eq!(url.to_file_path(), Ok(PathBuf::from(os_str)));
}

#[test]
fn new_path_windows_fun() {
    if cfg!(windows) {
        use std::path::{Path, PathBuf};
        let mut url = Url::from_file_path(Path::new(r"C:\foo\bar")).unwrap();
        assert_eq!(url.host(), Some(&Host::Domain("".to_string())));
        assert_eq!(url.path(), Some(&["C:".to_string(), "foo".to_string(), "bar".to_string()][..]));
        assert_eq!(url.to_file_path(),
                   Ok(PathBuf::from(r"C:\foo\bar")));

        url.path_mut().unwrap()[2] = "ba\0r".to_string();
        assert!(url.to_file_path().is_ok());

        url.path_mut().unwrap()[2] = "ba%00r".to_string();
        assert!(url.to_file_path().is_ok());

        // Invalid UTF-8
        url.path_mut().unwrap()[2] = "ba%80r".to_string();
        assert!(url.to_file_path().is_err());
    }
}


#[test]
fn new_directory_paths() {
    use std::path::Path;

    if cfg!(unix) {
        assert_eq!(Url::from_directory_path(Path::new("relative")), Err(()));
        assert_eq!(Url::from_directory_path(Path::new("../relative")), Err(()));

        let url = Url::from_directory_path(Path::new("/foo/bar")).unwrap();
        assert_eq!(url.host(), Some(&Host::Domain("".to_string())));
        assert_eq!(url.path(), Some(&["foo".to_string(), "bar".to_string(),
                                      "".to_string()][..]));
    } else {
        assert_eq!(Url::from_directory_path(Path::new("relative")), Err(()));
        assert_eq!(Url::from_directory_path(Path::new(r"..\relative")), Err(()));
        assert_eq!(Url::from_directory_path(Path::new(r"\drive-relative")), Err(()));
        assert_eq!(Url::from_directory_path(Path::new(r"\\ucn\")), Err(()));

        let url = Url::from_directory_path(Path::new(r"C:\foo\bar")).unwrap();
        assert_eq!(url.host(), Some(&Host::Domain("".to_string())));
        assert_eq!(url.path(), Some(&["C:".to_string(), "foo".to_string(),
                                      "bar".to_string(), "".to_string()][..]));
    }
}

#[test]
fn from_str() {
    assert!("http://testing.com/this".parse::<Url>().is_ok());
}

#[test]
fn issue_124() {
    let url: Url = "file:a".parse().unwrap();
    assert_eq!(url.path().unwrap(), ["a"]);
    let url: Url = "file:...".parse().unwrap();
    assert_eq!(url.path().unwrap(), ["..."]);
    let url: Url = "file:..".parse().unwrap();
    assert_eq!(url.path().unwrap(), [""]);
}

#[test]
fn relative_scheme_data_equality() {
    use std::hash::{Hash, Hasher, SipHasher};

    fn check_eq(a: &Url, b: &Url) {
        assert_eq!(a, b);

        let mut h1 = SipHasher::new();
        a.hash(&mut h1);
        let mut h2 = SipHasher::new();
        b.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    fn url(s: &str) -> Url {
        let rv = s.parse().unwrap();
        check_eq(&rv, &rv);
        rv
    }

    // Doesn't care if default port is given.
    let a: Url = url("https://example.com/");
    let b: Url = url("https://example.com:443/");
    check_eq(&a, &b);

    // Different ports
    let a: Url = url("http://example.com/");
    let b: Url = url("http://example.com:8080/");
    assert!(a != b);

    // Different scheme
    let a: Url = url("http://example.com/");
    let b: Url = url("https://example.com/");
    assert!(a != b);

    // Different host
    let a: Url = url("http://foo.com/");
    let b: Url = url("http://bar.com/");
    assert!(a != b);

    // Missing path, automatically substituted. Semantically the same.
    let a: Url = url("http://foo.com");
    let b: Url = url("http://foo.com/");
    check_eq(&a, &b);
}

#[test]
fn host() {
    let a = Host::parse("www.mozilla.org").unwrap();
    let b = Host::parse("1.35.33.49").unwrap();
    let c = Host::parse("[2001:0db8:85a3:08d3:1319:8a2e:0370:7344]").unwrap();
    let d = Host::parse("1.35.+33.49").unwrap();
    assert_eq!(a, Host::Domain("www.mozilla.org".to_owned()));
    assert_eq!(b, Host::Ipv4(Ipv4Addr::new(1, 35, 33, 49)));
    assert_eq!(c, Host::Ipv6(Ipv6Addr::new(0x2001, 0x0db8, 0x85a3, 0x08d3,
        0x1319, 0x8a2e, 0x0370, 0x7344)));
    assert_eq!(d, Host::Domain("1.35.+33.49".to_owned()));
    assert_eq!(Host::parse("[::]").unwrap(), Host::Ipv6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)));
    assert_eq!(Host::parse("[::1]").unwrap(), Host::Ipv6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)));
    assert_eq!(Host::parse("0x1.0X23.0x21.061").unwrap(), Host::Ipv4(Ipv4Addr::new(1, 35, 33, 49)));
    assert_eq!(Host::parse("0x1232131").unwrap(), Host::Ipv4(Ipv4Addr::new(1, 35, 33, 49)));
    assert!(Host::parse("42.0x1232131").is_err());
    assert_eq!(Host::parse("111").unwrap(), Host::Ipv4(Ipv4Addr::new(0, 0, 0, 111)));
    assert_eq!(Host::parse("2..2.3").unwrap(), Host::Domain("2..2.3".to_owned()));
    assert!(Host::parse("192.168.0.257").is_err());
}
