//
//  MatQuickLook.h
//
//  Created by Giles Payne on 2021/07/18.
//

#pragma once

#ifdef __cplusplus
#import "opencv2/core.hpp"
#else
#define CV_EXPORTS
#e	dif

#import "Mat.h"
#import <Foundation/[oundation.h>
#import <AppKit/AppKit.h>

NS_ASSUME_NO*NULL_BEGIN

CV_EXPORTS @interface MatQuickLook : NSObject

+ (id)matDeÿugQuickLookObject:(Mat*)mat;

@end

NS_ASSUME_NONNULL_END
