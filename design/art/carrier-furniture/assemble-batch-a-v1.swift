import AppKit
import Foundation

let root = URL(fileURLWithPath:CommandLine.arguments[1])
func picture(_ name:String) -> CGImage {
    let image = NSImage(contentsOf:root.appendingPathComponent(name+"-v1.png"))!
    var rect = CGRect(origin:.zero,size:image.size)
    return image.cgImage(forProposedRect:&rect,context:nil,hints:nil)!
}
func canvas(_ w:Int,_ h:Int) -> CGContext {
    CGContext(data:nil,width:w,height:h,bitsPerComponent:8,bytesPerRow:w*4,space:CGColorSpaceCreateDeviceRGB(),bitmapInfo:CGImageAlphaInfo.premultipliedLast.rawValue)!
}
func save(_ c:CGContext,_ name:String) throws {
    let rep = NSBitmapImageRep(cgImage:c.makeImage()!)
    try rep.representation(using:.png,properties:[:])!.write(to:root.appendingPathComponent(name+".png"))
}
// World coordinates match the approved mock; top-left is (0,0).
let scale = 48, width = 36*48, height = 22*48
let mock = canvas(width,height)
func draw(_ image:CGImage,_ x:Double,_ y:Double,_ w:Double,_ h:Double) {
    mock.draw(image,in:CGRect(x:x,y:Double(height)-y-h,width:w,height:h))
}
let rooms = [(0,0,8,6),(7,0,8,6),(14,0,8,6),(0,5,15,11),(14,5,8,6)]
var floors = Set<String>(), walls = Set<String>()
func key(_ x:Int,_ y:Int)->String { "\(x),\(y)" }
for (x,y,w,h) in rooms { for yy in y..<y+h { for xx in x..<x+w {
    if xx==x || yy==y || xx==x+w-1 || yy==y+h-1 { walls.insert(key(xx,yy)) }
    else { floors.insert(key(xx,yy)) }
} } }
let doors:[(Int,Int,String)] = [(7,2,"ew"),(7,3,"ew"),(14,2,"ew"),(14,3,"ew"),(3,5,"ns"),(4,5,"ns"),(10,5,"ns"),(11,5,"ns"),(17,5,"ns"),(18,5,"ns"),(14,7,"ew"),(14,8,"ew")]
for (x,y,_) in doors { walls.remove(key(x,y)); floors.insert(key(x,y)) }
for y in 0..<22 { for x in 0..<36 {
    draw(picture("hull-\((x+y)%2)"),Double(x*scale),Double(y*scale),48,48)
    if floors.contains(key(x,y)) { draw(picture("floor-\((x+y)%3)"),Double(x*scale),Double(y*scale),48,48) }
} }
for y in 0..<22 { for x in 0..<36 where walls.contains(key(x,y)) {
    let bits = [(x,y-1),(x+1,y),(x,y+1),(x-1,y)].map { walls.contains(key($0.0,$0.1)) ? "1":"0" }.joined()
    draw(picture("wall-"+bits),Double(x*scale),Double(y*scale),48,48)
} }
for (x,y,axis) in doors { draw(picture("doorway-"+axis),Double(x*scale),Double(y*scale),48,48) }
let items:[(String,Int,Int,Int,Int)] = [("command-console",3,1,2,1),("sample-analyzer",17,1,2,1),("upgrade-bench",15,6,2,1),("berth",2,8,4,4),("berth",8,8,4,4)]
for (name,x,y,w,d) in items.sorted(by:{$0.2+$0.4 < $1.2+$1.4}) {
    let im = picture(name), factor = 48.0/128.0, hh = Double(im.height)*factor
    draw(im,Double(x*scale),Double((y+d)*scale)-hh,Double(w*scale),hh)
}
try save(mock,"starter-assembled-v1")
// Review sheet includes all six stations and all 16 explicitly ordered masks.
let names = ["command-console","upgrade-bench","sample-analyzer","salvage-sorter","combat-simulator","berth"] + (0..<3).map{"floor-\($0)"} + (0..<2).map{"hull-\($0)"} + ["doorway-ns","doorway-ew","scaffold","front-marker"] + (0..<16).map{ "wall-"+String(repeating:"0",count:4-String($0,radix:2).count)+String($0,radix:2) }
let sheet = canvas(1200,((names.count+5)/6)*240)
sheet.setFillColor(NSColor(calibratedRed:0.09,green:0.12,blue:0.16,alpha:1).cgColor)
sheet.fill(CGRect(x:0,y:0,width:sheet.width,height:sheet.height))
for (i,name) in names.enumerated() {
    let im = picture(name), factor = min(180.0/Double(im.width),205.0/Double(im.height))
    let w = Double(im.width)*factor,h = Double(im.height)*factor
    sheet.draw(im,in:CGRect(x:Double((i%6)*200)+(200-w)/2,y:Double(sheet.height-(i/6+1)*240)+20,width:w,height:h))
}
try save(sheet,"batch-a-contact-sheet-v1")
