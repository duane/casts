extern crate feed_rs;
extern crate libc;
extern crate reqwest;

use feed_rs::model::{
  Content as RustContent, Entry as RustEntry, Feed as RustFeed, Image as RustImage,
  Link as RustLink,
};
use feed_rs::parser;
use std::ffi::{CStr, CString};
use std::fmt;
use std::ptr;

#[repr(C)]
pub struct Image {
  pub uri: *const libc::c_char,
  pub title: *const libc::c_char,
  pub link: *const Link,
  pub width: *const u32,
  pub height: *const u32,
  pub description: *const libc::c_char,
}

fn convert_image(image: RustImage) -> Image {
  Image {
    uri: CString::new(image.uri).unwrap().into_raw(),
    title: image
      .title
      .map(|s| CString::new(s).unwrap().into_raw() as *const i8)
      .unwrap_or(ptr::null()),
    link: image
      .link
      .map(|l| Box::into_raw(Box::new(convert_link(l))) as *const Link)
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

impl Drop for Image {
  fn drop(&mut self) {
    unsafe {
      if !self.uri.is_null() {
        Box::from_raw(self.uri as *mut i8);
      }
      if !self.title.is_null() {
        Box::from_raw(self.title as *mut i8);
      }
      if !self.link.is_null() {
        Box::from_raw(self.link as *mut Link);
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

#[repr(C)]
pub struct Link {
  pub href: *const libc::c_char,
  pub rel: *const libc::c_char,
  pub media_type: *const libc::c_char,
  pub href_lang: *const libc::c_char,
  pub title: *const libc::c_char,
  pub length: *const u32,
}

fn convert_link(link: RustLink) -> Link {
  Link {
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

impl Drop for Link {
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

#[repr(C)]
pub struct Content {
  pub body: *const libc::c_char,
  pub content_type: *const libc::c_char,
  pub length: *const u32,
  pub src: *const Link,
}

fn convert_content(content: RustContent) -> Content {
  Content {
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
      .map(|l| Box::into_raw(Box::new(convert_link(l))) as *const Link)
      .unwrap_or(ptr::null()),
  }
}

impl Drop for Content {
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
        Box::from_raw(self.src as *mut Link);
      }
    }
  }
}

#[repr(C)]
pub struct Entry {
  pub id: *const libc::c_char,
  pub title: *const libc::c_char,
  pub link: *const libc::c_char,
  pub summary: *const libc::c_char,
  pub content: *const Content,
}

fn convert_entry(entry: RustEntry) -> Entry {
  Entry {
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
      .map(|c| Box::into_raw(Box::new(convert_content(c))) as *const Content)
      .unwrap_or(ptr::null()),
  }
}

impl Drop for Entry {
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
        Box::from_raw(self.content as *mut Content);
      }
    }
  }
}

#[repr(C)]
pub struct Feed {
  pub title: *const libc::c_char,
  pub description: *const libc::c_char,
  pub logo: *const Image,
  pub entries: *const Entry,
  pub entries_count: u32,
}

impl Drop for Feed {
  fn drop(&mut self) {
    unsafe {
      if !self.title.is_null() {
        Box::from_raw(self.title as *mut i8);
      }
      if !self.description.is_null() {
        Box::from_raw(self.description as *mut i8);
      }
      if !self.entries.is_null() {
        Box::from_raw(self.entries as *mut Entry);
      }
    }
  }
}

#[derive(Debug)]
pub enum DownloadError {
  UrlError,
  RedirectError,
  StatusError(u16),
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
    }
    unreachable!()
  }
}

#[derive(Debug)]
pub enum ParseCastsError {
  ParseFeedError(parser::ParseFeedError),
  NotAnAudioCastFeed(String),
}

impl fmt::Display for ParseCastsError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ParseCastsError::ParseFeedError(error) => write!(f, "feed parse error: {:?}", error),
      ParseCastsError::NotAnAudioCastFeed(s) => write!(f, "not an audio cast feed: {}", s),
    }
  }
}

pub fn blocking_download_feed_rs(url: String) -> Result<String, DownloadError> {
  Ok(reqwest::blocking::get(&url)?.text()?)
}

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
pub extern "C" fn parse_feed(buffer: *const libc::c_char) -> *const Feed {
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
    .map(|i| Box::into_raw(Box::new(convert_image(i))) as *const Image)
    .unwrap_or(ptr::null());
  let entries: Vec<Entry> = parsed_rust_feed
    .entries
    .into_iter()
    .map(convert_entry)
    .collect();
  let entries_slice: Box<[Entry]> = entries.into_boxed_slice();
  let entries_count = entries_slice.len() as u32;
  let entries_ptr: *mut Entry = Box::into_raw(entries_slice) as _;
  Box::into_raw(Box::new(Feed {
    title,
    description,
    logo,
    entries_count,
    entries: entries_ptr,
  }))
}

#[no_mangle]
pub extern "C" fn free_feed(feed: *const Feed) {
  unsafe {
    Box::from_raw(feed as *mut Feed);
  }
}
