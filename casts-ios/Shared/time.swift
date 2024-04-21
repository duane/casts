//
//  time.swift
//  play
//
//  Created by Duane Bailey on 4/9/24.
//

import Foundation

let ONE_DAY_IN_SECONDS = 60 * 60 * 24;
let TWO_DAYS_IN_SECONDS = ONE_DAY_IN_SECONDS * 2;

func casualDuration(elapsed: TimeInterval) -> String {
    assert(elapsed >= 0.0);
    let seconds = Int(elapsed) % 60;
    let minutes = Int(elapsed / 60) % 60;
    let hours = Int(elapsed / 3600) % 24;
    let days = Int(elapsed) / ONE_DAY_IN_SECONDS;
    let years = Int(elapsed) / (ONE_DAY_IN_SECONDS * 365);
    return "\(hours)h \(minutes)m"
}

func casualTimeAgo(date: Date) -> String {
    let now = Date()
    let elapsed = now.timeIntervalSince(date)
    assert(elapsed >= 0.0);
    let seconds = Int(elapsed) % 60;
    let minutes = Int(elapsed / 60) % 60;
    let hours = Int(elapsed / 3600) % 24;
    let days = Int(elapsed) / ONE_DAY_IN_SECONDS;
    let years = Int(elapsed) / (ONE_DAY_IN_SECONDS * 365);

    if (years > 1) {
        return "\(years) years ago"
    }
    if (days > 1) {
        return "\(days) days ago"
    }
    if (days == 1) {
        return "yesterday"
    }
    if (hours > 1) {
        return "\(hours) hours ago"
    }
    if (minutes > 1) {
        return "\(minutes) minutes ago"
    }
    if (seconds > 1) {
        return "\(seconds) seconds ago"
    }
    return "now"
}
