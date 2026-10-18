import AppKit
import Foundation

// Inspect imported canvases and arrange a labelled preview; never slice source art.
let root = URL(fileURLWithPath: CommandLine.arguments[1])
let input = root.appendingPathComponent("batch-b-import-v1.json")
var entries = try JSONSerialization.jsonObject(with: Data(contentsOf: input)) as! [[String: Any]]
let sheet = NSImage(size: NSSize(width: 1200, height: 1200))
sheet.lockFocus()
NSColor(calibratedWhite: 0.16, alpha: 1).setFill()
NSRect(x: 0, y: 0, width: 1200, height: 1200).fill()
for i in entries.indices {
    let file = entries[i]["file"] as! String
    let image = NSImage(contentsOf: root.appendingPathComponent(file))!
    var rect = CGRect(origin: .zero, size: image.size)
    let cg = image.cgImage(forProposedRect: &rect, context: nil, hints: nil)!
    let rep = NSBitmapImageRep(cgImage: cg)
    let expected = entries[i]["size"] as! [Int]
    precondition(cg.width == expected[0] && cg.height == expected[1], "canvas: \(file)")
    var minX = cg.width, minY = cg.height, maxX = -1, maxY = -1
    for y in 0..<cg.height { for x in 0..<cg.width {
        if (rep.colorAt(x:x,y:y)?.alphaComponent ?? 0) > 0.02 {
            minX = min(minX,x); minY = min(minY,y)
            maxX = max(maxX,x); maxY = max(maxY,y)
        }
    } }
    precondition(minX >= 3 && minY >= 3 && maxX < cg.width-3 && maxY < cg.height-3, "clipped: \(file)")
    entries[i]["alpha_bbox"] = [minX,minY,maxX+1,maxY+1]
    let scale = min(200.0/Double(cg.width),245.0/Double(cg.height))
    let w = Double(cg.width)*scale, h = Double(cg.height)*scale
    let x = Double(i%5)*240+(240-w)/2, y = 1200-Double(i/5+1)*300+32
    image.draw(in: NSRect(x:x,y:y,width:w,height:h))
    (file as NSString).draw(at:NSPoint(x:Double(i%5)*240+8,y:y-20),withAttributes:[.font:NSFont.systemFont(ofSize:12),.foregroundColor:NSColor.white])
}
sheet.unlockFocus()
let png = NSBitmapImageRep(data:sheet.tiffRepresentation!)!.representation(using:.png,properties:[:])!
try png.write(to:root.appendingPathComponent("batch-b-contact-sheet-v1.png"))
let json = try JSONSerialization.data(withJSONObject:entries,options:[.prettyPrinted,.sortedKeys])
try json.write(to:input)
print("Verified \(entries.count) complete canvases with clear alpha margins.")

// A second sheet uses native 2x pixel scale for every item and a 112 px Pilot.
let pilotURL = root.appendingPathComponent("../demo-v3/pilot-8dir/frames/s/idle.png").standardizedFileURL
let pilot = NSImage(contentsOf: pilotURL)!
var pilotRect = CGRect(origin: .zero, size: pilot.size)
let pilotCG = pilot.cgImage(forProposedRect: &pilotRect, context: nil, hints: nil)!
let pilotRep = NSBitmapImageRep(cgImage: pilotCG)
var px = pilotCG.width, py = pilotCG.height, qx = 0, qy = 0
for y in 0..<pilotCG.height { for x in 0..<pilotCG.width {
    if (pilotRep.colorAt(x:x,y:y)?.alphaComponent ?? 0) > 0.02 {
        px = min(px,x); py = min(py,y); qx = max(qx,x); qy = max(qy,y)
    }
} }
let pilotWhole = pilotCG.cropping(to:CGRect(x:px,y:py,width:qx-px+1,height:qy-py+1))!
let pilotImage = NSImage(cgImage:pilotWhole,size:NSSize(width:pilotWhole.width,height:pilotWhole.height))
let scaleSheet = NSImage(size:NSSize(width:1536,height:2000))
scaleSheet.lockFocus()
NSColor(calibratedWhite:0.16,alpha:1).setFill()
NSRect(x:0,y:0,width:1536,height:2000).fill()
for i in entries.indices {
    let file = entries[i]["file"] as! String
    let size = entries[i]["size"] as! [Int]
    let x = Double(i%4)*384+8, y = 2000-Double(i/4+1)*400+24
    NSImage(contentsOf:root.appendingPathComponent(file))!.draw(in:NSRect(x:x,y:y,width:Double(size[0]),height:Double(size[1])))
    pilotImage.draw(in:NSRect(x:x+284,y:y+4,width:112.0*Double(pilotWhole.width)/Double(pilotWhole.height),height:112))
    (file as NSString).draw(at:NSPoint(x:x,y:y-18),withAttributes:[.font:NSFont.systemFont(ofSize:12),.foregroundColor:NSColor.white])
}
scaleSheet.unlockFocus()
try NSBitmapImageRep(data:scaleSheet.tiffRepresentation!)!.representation(using:.png,properties:[:])!.write(to:root.appendingPathComponent("batch-b-pilot-scale-v1.png"))
