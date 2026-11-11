#import <AppKit/AppKit.h>
#include <math.h>
extern void spark_center_window_controls(void *);
int main(void) {
  @autoreleasepool {
    NSApplication *app = [NSApplication sharedApplication];
    [app setActivationPolicy:NSApplicationActivationPolicyProhibited];
    NSWindow *window = [[NSWindow alloc] initWithContentRect:NSMakeRect(0,0,800,600) styleMask:(NSWindowStyleMaskTitled | NSWindowStyleMaskClosable | NSWindowStyleMaskMiniaturizable | NSWindowStyleMaskResizable | NSWindowStyleMaskFullSizeContentView) backing:NSBackingStoreBuffered defer:NO];
    window.titlebarAppearsTransparent = YES;
    window.titleVisibility = NSWindowTitleHidden;
    spark_center_window_controls((__bridge void *)window);
    NSDate *end = [NSDate dateWithTimeIntervalSinceNow:0.4];
    while (end.timeIntervalSinceNow > 0) [[NSRunLoop currentRunLoop] runUntilDate:[NSDate dateWithTimeIntervalSinceNow:0.02]];
    NSButton *close = [window standardWindowButton:NSWindowCloseButton];
    NSView *titlebar = close.superview.superview;
    NSRect barFrame = titlebar.frame;
    barFrame.size.height += 4;
    barFrame.origin.y -= 4;
    [titlebar setFrame:barFrame];
    [window setContentSize:NSMakeSize(900,700)];
    NSDate *resizeEnd = [NSDate dateWithTimeIntervalSinceNow:0.3];
    while (resizeEnd.timeIntervalSinceNow > 0) [[NSRunLoop currentRunLoop] runUntilDate:[NSDate dateWithTimeIntervalSinceNow:0.02]];
    for (int iteration = 0; iteration < 30; iteration++) {
      NSRect f = titlebar.frame;
      CGFloat height = close.frame.size.height + 22.0;
      f.size.height = height;
      f.origin.y = window.frame.size.height - height;
      [titlebar setFrame:f];
      NSArray<NSNumber *> *kinds = @[@(NSWindowCloseButton), @(NSWindowMiniaturizeButton), @(NSWindowZoomButton)];
      NSButton *middle = [window standardWindowButton:NSWindowMiniaturizeButton];
      CGFloat spacing = middle.frame.origin.x - close.frame.origin.x;
      for (NSUInteger i=0; i<kinds.count; i++) {
        NSButton *button = [window standardWindowButton:kinds[i].unsignedIntegerValue];
        NSPoint origin = button.frame.origin;
        origin.x = 16.0 + i * spacing;
        [button setFrameOrigin:origin];
      }
      [[NSRunLoop currentRunLoop] runUntilDate:[NSDate dateWithTimeIntervalSinceNow:0.02]];
    }
    NSView *content = window.contentView;
    for (NSNumber *kind in @[@(NSWindowCloseButton), @(NSWindowMiniaturizeButton), @(NSWindowZoomButton)]) {
      NSButton *button = [window standardWindowButton:kind.unsignedIntegerValue];
      NSRect frame = [button.superview convertRect:button.frame toView:content];
      CGFloat offset = content.isFlipped ? NSMidY(frame)-NSMinY(content.bounds) : NSMaxY(content.bounds)-NSMidY(frame);
      printf("button=%lu center-from-top=%.2f\n",(unsigned long)kind.unsignedIntegerValue,offset);
      if (fabs(offset-28.0)>0.5) return 1;
    }
    return 0;
  }
}
