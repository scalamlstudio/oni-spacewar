import AppKit
import Foundation

// Resize whole generated transparent artwork; never extract art from a scene.
// Usage: swift prepare-batch-a-v1.swift <source> <destination> <width> <height> <tile|object>
let a = CommandLine.arguments
guard a.count == 6, let image = NSImage(contentsOfFile: a[1]), let w = Int(a[3]), let h = Int(a[4]) else { fatalError("arguments") }
var originalRect = CGRect(origin: .zero, size: image.size)
guard let cg = image.cgImage(forProposedRect: &originalRect, context: nil, hints: nil) else { fatalError("image") }
let rep = NSBitmapImageRep(cgImage: cg)
var minX = cg.width, minY = cg.height, maxX = -1, maxY = -1
for y in 0..<cg.height { for x in 0..<cg.width {
    if (rep.colorAt(x: x, y: y)?.alphaComponent ?? 0) > 0.02 {
        minX = min(minX,x); minY = min(minY,y); maxX = max(maxX,x); maxY = max(maxY,y)
    }
} }
guard maxX >= minX else { fatalError("empty image") }
let space = CGColorSpaceCreateDeviceRGB()
guard let ctx = CGContext(data:nil,width:w,height:h,bitsPerComponent:8,bytesPerRow:w*4,space:space,bitmapInfo:CGImageAlphaInfo.premultipliedLast.rawValue) else { fatalError("context") }
ctx.interpolationQuality = .high
if a[5] == "tile" {
    ctx.draw(cg,in:CGRect(x:0,y:0,width:w,height:h))
} else {
    // Crop only already-transparent padding, preserve aspect ratio and the whole subject.
    let bounds = CGRect(x:minX,y:minY,width:maxX-minX+1,height:maxY-minY+1)
    guard let whole = cg.cropping(to:bounds) else { fatalError("bounds") }
    let scale = min(Double(w-8)/Double(whole.width),Double(h-8)/Double(whole.height))
    let dw = Double(whole.width)*scale, dh = Double(whole.height)*scale
    ctx.draw(whole,in:CGRect(x:(Double(w)-dw)/2,y:4,width:dw,height:dh))
}
let out = NSBitmapImageRep(cgImage:ctx.makeImage()!)
try out.representation(using:.png,properties:[:])!.write(to:URL(fileURLWithPath:a[2]))
print("\(a[2]): \(w)x\(h), original alpha bbox \(minX),\(minY)–\(maxX),\(maxY)")
