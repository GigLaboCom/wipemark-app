// sckrec.swift — record one window, and nothing else, to an H.264 .mp4.
//
// What it is for: the clips in media/out/clips/ must show Wipemark's own
// window and nothing that happens to be on the screen — another program's
// window passing over it, a notification, the menu bar. A screen rectangle
// would take all of those in. ScreenCaptureKit's desktop-independent window
// filter reads the window's own content even while something covers it, so
// the clip holds that window alone.
//
// How to build: swiftc -O -o sckrec media/screenshots/sckrec.swift
// How to run:   sckrec <CGWindowID> <seconds> <out.mp4> [fps]
//               (window ids from `python3 media/screenshots/wm.py list <pid>`)
// Needs:        macOS 14 or later; Screen Recording for the terminal it runs from.
// Output:       the window at its backing scale (2x on Retina), 30 fps by
//               default; frames come only when the window changes, so a quiet
//               stretch is one long frame.
import AppKit
import AVFoundation
import CoreMedia
import Foundation
import ScreenCaptureKit

final class Recorder: NSObject, SCStreamOutput, SCStreamDelegate {
    let writer: AVAssetWriter
    let input: AVAssetWriterInput
    var started = false
    var last = CMTime.invalid
    var frames = 0

    init(url: URL, width: Int, height: Int) throws {
        writer = try AVAssetWriter(outputURL: url, fileType: .mp4)
        input = AVAssetWriterInput(mediaType: .video, outputSettings: [
            AVVideoCodecKey: AVVideoCodecType.h264,
            AVVideoWidthKey: width,
            AVVideoHeightKey: height,
            AVVideoCompressionPropertiesKey: [
                AVVideoAverageBitRateKey: 6_000_000,
                AVVideoProfileLevelKey: AVVideoProfileLevelH264HighAutoLevel,
            ],
        ])
        input.expectsMediaDataInRealTime = true
        writer.add(input)
        writer.startWriting()
    }

    func stream(_ stream: SCStream, didOutputSampleBuffer buffer: CMSampleBuffer, of type: SCStreamOutputType) {
        guard type == .screen, buffer.isValid else { return }
        guard let attachments = CMSampleBufferGetSampleAttachmentsArray(buffer, createIfNecessary: false)
            as? [[SCStreamFrameInfo: Any]],
            let raw = attachments.first?[.status] as? Int,
            SCFrameStatus(rawValue: raw) == .complete
        else { return }
        let at = CMSampleBufferGetPresentationTimeStamp(buffer)
        if !started {
            writer.startSession(atSourceTime: at)
            started = true
        }
        if input.isReadyForMoreMediaData {
            input.append(buffer)
            last = at
            frames += 1
        }
    }

    func stream(_ stream: SCStream, didStopWithError error: Error) {
        FileHandle.standardError.write("stream stopped: \(error)\n".data(using: .utf8)!)
    }
}

let args = CommandLine.arguments
guard args.count >= 4, let id = UInt32(args[1]), let seconds = Double(args[2]) else {
    FileHandle.standardError.write("usage: sckrec <CGWindowID> <seconds> <out.mp4> [fps]\n".data(using: .utf8)!)
    exit(2)
}
// A command-line program has no window server connection until something asks
// for one; ScreenCaptureKit asserts without it.
_ = NSApplication.shared
let url = URL(fileURLWithPath: args[3])
let fps = args.count > 4 ? Int32(args[4]) ?? 30 : 30
try? FileManager.default.removeItem(at: url)

let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: false)
guard let window = content.windows.first(where: { $0.windowID == id }) else {
    FileHandle.standardError.write("no window \(id)\n".data(using: .utf8)!)
    exit(1)
}
let filter = SCContentFilter(desktopIndependentWindow: window)
let scale = CGFloat(filter.pointPixelScale)
let width = Int(filter.contentRect.width * scale) / 2 * 2
let height = Int(filter.contentRect.height * scale) / 2 * 2

let config = SCStreamConfiguration()
config.width = width
config.height = height
config.minimumFrameInterval = CMTime(value: 1, timescale: fps)
config.pixelFormat = kCVPixelFormatType_32BGRA
config.showsCursor = true
config.queueDepth = 8
config.ignoreShadowsSingleWindow = true

let recorder = try Recorder(url: url, width: width, height: height)
let stream = SCStream(filter: filter, configuration: config, delegate: recorder)
try stream.addStreamOutput(recorder, type: .screen, sampleHandlerQueue: DispatchQueue(label: "sckrec"))
try await stream.startCapture()
try await Task.sleep(nanoseconds: UInt64(seconds * 1_000_000_000))
try? await stream.stopCapture()

// Hold the last frame to the end, so a quiet ending is not cut short.
let end = CMClockGetTime(CMClockGetHostTimeClock())
recorder.input.markAsFinished()
if recorder.started {
    recorder.writer.endSession(atSourceTime: CMTimeMaximum(end, recorder.last))
}
await recorder.writer.finishWriting()
print("\(args[3]) \(width)x\(height) \(recorder.frames) frames")
