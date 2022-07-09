#![deny(missing_docs)]
#![deny(missing_doc_code_examples)]

//! Utility functions for downloading and parsing feeds.
//!
//! This library is intended to be consumed both from rust and from c.

/// Downloading and streaming
pub mod net;

/// actors
pub mod actors;

/// Feed handling & utilities.
pub mod feeds;

/// Event sourcing definitions.
pub mod aggregate;
/// commands
pub mod commands;
/// events
pub mod events;

/// A useful object for tracking downloads.
pub mod download_object;

/// The domain model for working with feeds.
pub mod model;

/// Utilities.
pub mod util;

/// console driver
//pub mod console;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate anyhow;
extern crate event_source;
extern crate feed_rs;
extern crate libc;
extern crate quick_xml;
extern crate reqwest;

extern crate async_trait;
// use log::info;

use feed_rs::model::{
  Content as RustContent, Entry as RustEntry, Feed as RustFeed, Image as RustImage,
  Link as RustLink,
};
use feed_rs::parser;
use std::error::Error;
use std::ffi::{CStr, CString};
use std::fmt;
use std::ptr;
use url::Url;

/// Representing an Image as seen from a feed.
#[repr(C)]
pub struct CImage {
  /// The URL where the image may be found.
  pub url: *const libc::c_char,

  /// The Title of the image.
  pub title: *const libc::c_char,

  /// The Link representing the image.
  pub link: *const CLink,

  /// The width of the image in pixels.
  pub width: *const u32,

  /// The height of the image in pixels.
  pub height: *const u32,

  /// A description of the image.
  pub description: *const libc::c_char,
}

fn convert_image(image: RustImage) -> CImage {
  CImage {
    url: CString::new(image.uri).unwrap().into_raw(),
    title: image
      .title
      .map(|s| CString::new(s).unwrap().into_raw() as *const i8)
      .unwrap_or(ptr::null()),
    link: image
      .link
      .map(|l| Box::into_raw(Box::new(convert_link(l))) as *const CLink)
      .unwrap_or(ptr::null()),
    width: image
      .width
      .map(|width| Box::into_raw(Box::new(width)) as *const u32)
      .unwrap_or(ptr::null()),
    height: image
      .height
      .map(|height| Box::into_raw(Box::new(height)) as *const u32)
      .unwrap_or(ptr::null()),
    description: image
      .description
      .map(|d| CString::new(d).unwrap().into_raw() as *const i8)
      .unwrap_or(ptr::null()),
  }
}

impl Drop for CImage {
  fn drop(&mut self) {
    unsafe {
      if !self.url.is_null() {
        Box::from_raw(self.url as *mut i8);
      }
      if !self.title.is_null() {
        Box::from_raw(self.title as *mut i8);
      }
      if !self.link.is_null() {
        Box::from_raw(self.link as *mut CLink);
      }
      if !self.width.is_null() {
        Box::from_raw(self.width as *mut u32);
      }
      if !self.height.is_null() {
        Box::from_raw(self.height as *mut u32);
      }
      if !self.description.is_null() {
        Box::from_raw(self.description as *mut u32);
      }
    }
  }
}

/// Representing a Link object as seen from a podcast feed.
#[repr(C)]
pub struct CLink {
  /// The URL of the link.
  pub href: *const libc::c_char,

  /// The rel of the link.
  pub rel: *const libc::c_char,

  /// The media type of the the linked-to content.
  pub media_type: *const libc::c_char,

  /// The language linked to by href.
  pub href_lang: *const libc::c_char,

  /// The title of the link.
  pub title: *const libc::c_char,

  /// The length of the content in bytes.
  pub length: *const u32,
}

fn convert_link(link: RustLink) -> CLink {
  CLink {
    href: CString::new(link.href).unwrap().into_raw() as *const i8,
    rel: link
      .rel
      .map(|s| CString::new(s).unwrap().into_raw() as *const i8)
      .unwrap_or(ptr::null()),
    media_type: link
      .media_type
      .map(|s| CString::new(s).unwrap().into_raw() as *const i8)
      .unwrap_or(ptr::null()),
    href_lang: link
      .href_lang
      .map(|s| CString::new(s).unwrap().into_raw() as *const i8)
      .unwrap_or(ptr::null()),
    title: link
      .title
      .map(|s| CString::new(s).unwrap().into_raw() as *const i8)
      .unwrap_or(ptr::null()),
    length: link
      .length
      .map(|i| Box::into_raw(Box::new(i as u32)) as *const u32)
      .unwrap_or(ptr::null()),
  }
}

impl Drop for CLink {
  fn drop(&mut self) {
    unsafe {
      if !self.href.is_null() {
        Box::from_raw(self.href as *mut i8);
      }
      if !self.rel.is_null() {
        Box::from_raw(self.rel as *mut i8);
      }
      if !self.media_type.is_null() {
        Box::from_raw(self.media_type as *mut i8);
      }
      if !self.href_lang.is_null() {
        Box::from_raw(self.href_lang as *mut i8);
      }
      if !self.title.is_null() {
        Box::from_raw(self.title as *mut i8);
      }
      if !self.length.is_null() {
        Box::from_raw(self.length as *mut u32);
      }
    }
  }
}

/// Representing content as exposed by feeds-rs
#[repr(C)]
pub struct CContent {
  /// The body of the content.
  pub body: *const libc::c_char,

  /// The content type of the content.
  pub content_type: *const libc::c_char,

  /// The length of the content in bytes.
  pub length: *const u32,

  /// A link where the content may be found.
  pub src: *const CLink,
}

fn convert_content(content: RustContent) -> CContent {
  CContent {
    body: content
      .body
      .map(|s| CString::new(s).unwrap().into_raw() as *const i8)
      .unwrap_or(ptr::null()),
    content_type: CString::new(content.content_type.essence_str())
      .unwrap()
      .into_raw() as *const i8,
    length: content
      .length
      .map(|i| Box::into_raw(Box::new(i as u32)) as *const u32)
      .unwrap_or(ptr::null()),
    src: content
      .src
      .map(|l| Box::into_raw(Box::new(convert_link(l))) as *const CLink)
      .unwrap_or(ptr::null()),
  }
}

impl Drop for CContent {
  fn drop(&mut self) {
    unsafe {
      if !self.body.is_null() {
        Box::from_raw(self.body as *mut i8);
      }
      if !self.content_type.is_null() {
        Box::from_raw(self.content_type as *mut i8);
      }
      if !self.length.is_null() {
        Box::from_raw(self.length as *mut u32);
      }
      if !self.src.is_null() {
        Box::from_raw(self.src as *mut CLink);
      }
    }
  }
}

/// An entry in the podcast.
#[repr(C)]
pub struct CEntry {
  /// a GUID assigned by feeds-rs if not specified by the feed.
  pub id: *const libc::c_char,
  /// The title of the entry.
  pub title: *const libc::c_char,
  /// The link of the entry.
  pub link: *const libc::c_char,
  /// The summary of the entry (as text/html).
  pub summary: *const libc::c_char,

  /// The content of the entry.
  pub content: *const CContent,
}

fn convert_entry(entry: RustEntry) -> CEntry {
  CEntry {
    id: CString::new(entry.id).unwrap().into_raw() as *const i8,
    title: entry
      .title
      .map(|f| f.content)
      .map(|s| CString::new(s).unwrap().into_raw() as *const i8)
      .unwrap_or(ptr::null()),
    summary: entry
      .summary
      .map(|f| f.content)
      .map(|s| CString::new(s).unwrap().into_raw() as *const i8)
      .unwrap_or(ptr::null()),
    link: ptr::null(),
    content: entry
      .content
      .map(|c| Box::into_raw(Box::new(convert_content(c))) as *const CContent)
      .unwrap_or(ptr::null()),
  }
}

impl Drop for CEntry {
  fn drop(&mut self) {
    unsafe {
      Box::from_raw(self.id as *mut i8);
      if !self.title.is_null() {
        Box::from_raw(self.title as *mut i8);
      }
      if !self.link.is_null() {
        Box::from_raw(self.link as *mut i8);
      }
      if !self.summary.is_null() {
        Box::from_raw(self.summary as *mut i8);
      }
      if !self.content.is_null() {
        Box::from_raw(self.content as *mut CContent);
      }
    }
  }
}

/// A feed according to feeds-rs.
#[repr(C)]
pub struct CFeed {
  /// A title for the feed (optional).
  pub title: *const libc::c_char,

  /// A description for the feed (optional).
  pub description: *const libc::c_char,

  /// An image to display with the feed (optional).
  pub logo: *const CImage,

  /// Entries for the feed.
  pub entries: *const CEntry,

  /// Total number of parsed entries.
  pub entries_count: u32,
}

// impl From<CFeed> for immutable_feed::Feed {
//   fn from(feed: CFeed) -> immutable_feed::Feed {
//     immutable_feed::Feed {
//       title: feed.title,
//       url: feed.url,
//       description: feed.description,
//       updated_at: feed.updated_at,
//       published_at: feed.published_at,
//       entries: feed.entries.iter().into_iter().map(|e|
//         immutable_feed::Entry{
//           guid: e.guid,
//           url: e.url,
//           title: e.url,
//           summary: e.summary,
//           updated_at: e.updated_at,
//           published_at: e.published_at,
//         }).collect(),
//     }
//   }
// }

impl Drop for CFeed {
  fn drop(&mut self) {
    unsafe {
      if !self.title.is_null() {
        Box::from_raw(self.title as *mut i8);
      }
      if !self.description.is_null() {
        Box::from_raw(self.description as *mut i8);
      }
      if !self.entries.is_null() {
        Box::from_raw(self.entries as *mut CEntry);
      }
    }
  }
}

/// An error that prevented a successful download.
#[derive(Debug)]
pub enum DownloadError {
  /// There was a problem with the URL.
  UrlError,

  /// There was a redirect response.
  RedirectError,

  /// There was an unexpected status code response to the error.
  StatusError(u16),

  /// There was a timeout.
  TimeoutError,
}

impl fmt::Display for DownloadError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      DownloadError::UrlError => write!(f, "not a valid url"),
      DownloadError::RedirectError => write!(f, "redirect"),
      DownloadError::StatusError(status) => write!(f, "unexpected http status {}", status),
      DownloadError::TimeoutError => write!(f, "timeout"),
    }
  }
}

impl std::convert::From<reqwest::Error> for DownloadError {
  fn from(err: reqwest::Error) -> Self {
    if err.is_builder() {
      return DownloadError::UrlError;
    } else if err.is_redirect() {
      return DownloadError::RedirectError;
    } else if err.is_status() {
      return DownloadError::StatusError(err.status().unwrap().as_u16());
    } else if err.is_timeout() {
      return DownloadError::TimeoutError;
    } else {
      panic!("{}", err);
    }
  }
}

impl Error for DownloadError {}

/// A parse error.
#[derive(Debug)]
pub enum ParseCastsError {
  /// There was an error with the form of the feed.
  ParseFeedError(parser::ParseFeedError),

  /// There was an error interpreting the feed as a podcast.
  NotAnAudioCastFeed(String),
}

impl Error for ParseCastsError {}

impl fmt::Display for ParseCastsError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ParseCastsError::ParseFeedError(error) => write!(f, "feed parse error: {:?}", error),
      ParseCastsError::NotAnAudioCastFeed(s) => write!(f, "not an audio cast feed: {}", s),
    }
  }
}

/// Block the thread until the feed has been downloaded, or there is an error.
pub fn blocking_download_feed_rs(url: &Url) -> Result<String, DownloadError> {
  Ok(reqwest::blocking::get(&url.to_string())?.text()?)
}

/// Parse a string containing a podcast feed.
pub fn parse_feed_rs(buffer: &str) -> Result<RustFeed, ParseCastsError> {
  let parsed_rust_feed = match parser::parse(buffer.as_bytes()) {
    Ok(rust_feed) => rust_feed,
    Err(error) => return Err(ParseCastsError::ParseFeedError(error)),
  };

  if parsed_rust_feed.title.is_none() || parsed_rust_feed.description.is_none() {
    return Err(ParseCastsError::NotAnAudioCastFeed(buffer.to_string()));
  }

  Ok(parsed_rust_feed)
}

#[no_mangle]
/// Parse a string containing a podcast feed.
///
/// The buffer should be allocated by the calling stack.
/// The caller is responsible for freeing the returned `Feed` with `free_feed`.
pub extern "C" fn parse_feed(buffer: *const libc::c_char) -> *const CFeed {
  let slice = unsafe { CStr::from_ptr(buffer) };
  let str_slice: &str = slice.to_str().unwrap();
  let parsed_rust_feed = parse_feed_rs(str_slice).unwrap();

  let title = parsed_rust_feed
    .title
    .map(|f| f.content)
    .map(|s| CString::new(s).unwrap().into_raw() as *const i8)
    .unwrap_or(ptr::null());
  let description = parsed_rust_feed
    .description
    .map(|f| f.content)
    .map(|s| CString::new(s).unwrap().into_raw() as *const i8)
    .unwrap_or(ptr::null());
  let logo = parsed_rust_feed
    .logo
    .map(|i| Box::into_raw(Box::new(convert_image(i))) as *const CImage)
    .unwrap_or(ptr::null());
  let entries: Vec<CEntry> = parsed_rust_feed
    .entries
    .into_iter()
    .map(convert_entry)
    .collect();
  let entries_slice: Box<[CEntry]> = entries.into_boxed_slice();
  let entries_count = entries_slice.len() as u32;
  let entries_ptr: *mut CEntry = Box::into_raw(entries_slice) as _;
  Box::into_raw(Box::new(CFeed {
    title,
    description,
    logo,
    entries_count,
    entries: entries_ptr,
  }))
}

/// Call when done with a `Feed` to free its allocated memory.
#[no_mangle]
pub extern "C" fn free_feed(feed: *const CFeed) {
  unsafe {
    Box::from_raw(feed as *mut CFeed);
  }
}
