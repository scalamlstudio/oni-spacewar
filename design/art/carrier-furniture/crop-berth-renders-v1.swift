import Foundation
import ImageIO
import CoreGraphics
let root = URL(fileURLWithPath: CommandLine.arguments[1])
for url in try FileManager.default.contentsOfDirectory(at: root, includingPropertiesForKeys: nil) where url.lastPathComponent.hasPrefix("berth-floor-") && url.pathExtension == "svg" {
 let svg = try String(contentsOf: url, encoding: .utf8)
 let regex = try NSRegularExpression(pattern: "width=\"([0-9]+)\" height=\"([0-9]+)\"")
 let ns = svg as NSString
 let match = regex.firstMatch(in: svg, range: NSRange(location: 0,length: ns.length))!
 let w = Int(ns.substring(with: match.range(at: 1)))!
 let h = Int(ns.substring(with: match.range(at: 2)))!
 let png = url.deletingPathExtension().appendingPathExtension("png")
 let source = CGImageSourceCreateWithURL(png as CFURL,nil)!
 let image = CGImageSourceCreateImageAtIndex(source,0,nil)!
 let cropped = image.cropping(to: CGRect(x:0,y:0,width:w,height:h))!
 let dest = CGImageDestinationCreateWithURL(png as CFURL,"public.png" as CFString,1,nil)!
 CGImageDestinationAddImage(dest,cropped,nil)
 precondition(CGImageDestinationFinalize(dest))
 precondition(cropped.width == w && cropped.height == h)
}
print("PASS: all exported canvas dimensions")
