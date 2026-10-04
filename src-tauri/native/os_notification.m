#import <Foundation/Foundation.h>
#import <UserNotifications/UserNotifications.h>

typedef void (*SparkClickCb)(const char *scheme);

static SparkClickCb g_click_cb = NULL;
static id g_delegate = nil;

@interface SparkUNDelegate : NSObject <UNUserNotificationCenterDelegate>
@end

@implementation SparkUNDelegate
- (void)userNotificationCenter:(UNUserNotificationCenter *)center
       willPresentNotification:(UNNotification *)notification
         withCompletionHandler:(void (^)(UNNotificationPresentationOptions))completionHandler {
  completionHandler(UNNotificationPresentationOptionList | UNNotificationPresentationOptionBanner |
                    UNNotificationPresentationOptionSound);
}

- (void)userNotificationCenter:(UNUserNotificationCenter *)center
didReceiveNotificationResponse:(UNNotificationResponse *)response
         withCompletionHandler:(void (^)(void))completionHandler {
  NSString *scheme = response.notification.request.content.userInfo[@"scheme"];
  if (scheme.length > 0 && g_click_cb) {
    g_click_cb(scheme.UTF8String);
  }
  if (completionHandler) {
    completionHandler();
  }
}
@end

static int wait_off_main(int (^work)(void)) {
  if (![NSThread isMainThread]) {
    return work();
  }
  __block int result = -1;
  dispatch_sync(dispatch_get_global_queue(QOS_CLASS_USER_INITIATED, 0), ^{
    result = work();
  });
  return result;
}

// Unpackaged `tauri dev` / raw Mach-O has no CFBundleIdentifier.
// Calling the UN center API then asserts and abort()s.
static int spark_un_unavailable(void) {
  NSString *bid = [[NSBundle mainBundle] bundleIdentifier];
  return bid.length > 0 ? 0 : -2;
}

int spark_un_request_authorization(void) {
  int unavailable = spark_un_unavailable();
  if (unavailable) {
    return unavailable;
  }
  return wait_off_main(^{
    dispatch_semaphore_t sem = dispatch_semaphore_create(0);
    __block int granted = 0;
    UNUserNotificationCenter *center = [UNUserNotificationCenter currentNotificationCenter];
    UNAuthorizationOptions options = UNAuthorizationOptionAlert | UNAuthorizationOptionSound;
    [center requestAuthorizationWithOptions:options
                          completionHandler:^(BOOL ok, NSError *error) {
                            granted = (error == nil && ok) ? 1 : 0;
                            dispatch_semaphore_signal(sem);
                          }];
    dispatch_semaphore_wait(sem, DISPATCH_TIME_FOREVER);
    return granted;
  });
}

int spark_un_read_authorization(void) {
  int unavailable = spark_un_unavailable();
  if (unavailable) {
    return unavailable;
  }
  return wait_off_main(^{
    dispatch_semaphore_t sem = dispatch_semaphore_create(0);
    __block int granted = 0;
    UNUserNotificationCenter *center = [UNUserNotificationCenter currentNotificationCenter];
    [center getNotificationSettingsWithCompletionHandler:^(UNNotificationSettings *settings) {
      UNAuthorizationStatus status = settings.authorizationStatus;
      granted = (status == UNAuthorizationStatusAuthorized ||
                 status == UNAuthorizationStatusProvisional)
                    ? 1
                    : 0;
      dispatch_semaphore_signal(sem);
    }];
    dispatch_semaphore_wait(sem, DISPATCH_TIME_FOREVER);
    return granted;
  });
}

int spark_un_deliver(const char *title, const char *body, const char *scheme) {
  int unavailable = spark_un_unavailable();
  if (unavailable) {
    return unavailable;
  }
  if (!title || !body || !scheme) {
    return -1;
  }
  NSString *titleStr = [NSString stringWithUTF8String:title];
  NSString *bodyStr = [NSString stringWithUTF8String:body];
  NSString *schemeStr = [NSString stringWithUTF8String:scheme];
  if (!titleStr || !bodyStr || !schemeStr) {
    return -1;
  }
  return wait_off_main(^{
    dispatch_semaphore_t sem = dispatch_semaphore_create(0);
    __block int ok = -1;
    UNMutableNotificationContent *content = [UNMutableNotificationContent new];
    content.title = titleStr;
    content.body = bodyStr;
    content.userInfo = @{@"scheme" : schemeStr};
    content.sound = [UNNotificationSound defaultSound];
    NSString *ident = [NSUUID UUID].UUIDString;
    UNNotificationRequest *request = [UNNotificationRequest requestWithIdentifier:ident
                                                                          content:content
                                                                          trigger:nil];
    UNUserNotificationCenter *center = [UNUserNotificationCenter currentNotificationCenter];
    [center addNotificationRequest:request
             withCompletionHandler:^(NSError *error) {
               ok = error == nil ? 1 : -1;
               dispatch_semaphore_signal(sem);
             }];
    dispatch_semaphore_wait(sem, DISPATCH_TIME_FOREVER);
    return ok;
  });
}

void spark_un_set_click_callback(SparkClickCb cb) {
  g_click_cb = cb;
  if (spark_un_unavailable()) {
    return;
  }
  static dispatch_once_t once;
  dispatch_once(&once, ^{
    g_delegate = [SparkUNDelegate new];
    [UNUserNotificationCenter currentNotificationCenter].delegate = g_delegate;
  });
}
