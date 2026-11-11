#import <AppKit/AppKit.h>
#import <objc/runtime.h>

// Must match --app-header-height in frontend/app.css, in logical points.
static const CGFloat SparkHeaderHeight = 56.0;
static const void *SparkWindowControlsKey = &SparkWindowControlsKey;

@interface SparkWindowControls : NSObject
@property(nonatomic, weak) NSWindow *window;
@property(nonatomic, weak) NSView *observedTitlebar;
@property(nonatomic, weak) NSView *observedButtonParent;
@property(nonatomic) BOOL scheduled;
@end

@implementation SparkWindowControls

- (void)scheduleAlignment:(NSNotification *)notification {
    (void)notification;
    if (self.scheduled) return;
    self.scheduled = YES;
    dispatch_async(dispatch_get_main_queue(), ^{
        self.scheduled = NO;
        [self align];
    });
}

- (void)observeView:(NSView *)view replacing:(NSView *)oldView {
    if (view == oldView) return;
    NSNotificationCenter *center = NSNotificationCenter.defaultCenter;
    if (oldView) [center removeObserver:self name:NSViewFrameDidChangeNotification object:oldView];
    if (view) {
        view.postsFrameChangedNotifications = YES;
        [center addObserver:self selector:@selector(scheduleAlignment:)
                       name:NSViewFrameDidChangeNotification object:view];
    }
}

- (void)align {
    NSWindow *window = self.window;
    NSView *content = window.contentView;
    NSButton *close = [window standardWindowButton:NSWindowCloseButton];
    NSView *parent = close.superview;
    if (!window || !content || !close || !parent) return;

    NSView *titlebar = parent.superview;
    [self observeView:titlebar replacing:self.observedTitlebar];
    [self observeView:parent replacing:self.observedButtonParent];
    self.observedTitlebar = titlebar;
    self.observedButtonParent = parent;

    NSRect bounds = content.bounds;
    CGFloat target = content.isFlipped ? NSMinY(bounds) + SparkHeaderHeight / 2.0
                                       : NSMaxY(bounds) - SparkHeaderHeight / 2.0;
    for (NSNumber *kind in @[@(NSWindowCloseButton), @(NSWindowMiniaturizeButton), @(NSWindowZoomButton)]) {
        NSButton *button = [window standardWindowButton:kind.unsignedIntegerValue];
        NSView *buttonParent = button.superview;
        if (!button || !buttonParent) continue;
        NSRect frame = button.frame;
        NSRect inContent = [buttonParent convertRect:frame toView:content];
        NSPoint targetPoint = [buttonParent convertPoint:NSMakePoint(NSMidX(inContent), target)
                                               fromView:content];
        CGFloat newY = targetPoint.y - NSHeight(frame) / 2.0;
        if (fabs(NSMinY(frame) - newY) > 0.25) {
            frame.origin.y = newY;
            [button setFrameOrigin:frame.origin];
        }
    }
}

@end

void spark_center_window_controls(void *rawWindow) {
    NSWindow *window = (__bridge NSWindow *)rawWindow;
    if (!window) return;
    dispatch_async(dispatch_get_main_queue(), ^{
        SparkWindowControls *controls = objc_getAssociatedObject(window, SparkWindowControlsKey);
        if (!controls) {
            controls = [SparkWindowControls new];
            controls.window = window;
            objc_setAssociatedObject(window, SparkWindowControlsKey, controls, OBJC_ASSOCIATION_RETAIN_NONATOMIC);
            NSNotificationCenter *center = NSNotificationCenter.defaultCenter;
            for (NSNotificationName name in @[NSWindowDidResizeNotification, NSWindowDidEnterFullScreenNotification, NSWindowDidExitFullScreenNotification]) {
                [center addObserver:controls selector:@selector(scheduleAlignment:) name:name object:window];
            }
        }
        [controls align];
        [controls scheduleAlignment:nil];
    });
}
