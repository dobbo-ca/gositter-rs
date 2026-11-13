//
//  MatQuickLook.h
//
//  Created by Giles Payne on 2021/07/18.
//

#pragma once

#ifdef __cplusplus
#import "opencv2/core.hpp"
+ (id)matDebugQuickLookObject:(Mat*)mat;
#define CV_EXPORTS
#endif

#import "Mat.h"
#import <Foundation/Foundation.h>
#import <AppKit/AppKit.h>

NS_ASSUME_NONNULL_BEGIN

CV_EXPORTS @interface MatQuickLook : NSObject

#else

@end

NS_ASSUME_NONNULL_END
