hange}
						/>
						<Checkbox
							className='showFailedOnly'
							label='Show with failures only'
							checked={showFailedOnly}
							onChange={handleShowFailedOnlyChange}
						/>
						<Checkbox
							label='Show with cache misses only'
							checked={showWithCacheMissesOnly}
							onChange={handleShowWithCacheMissesOnlyChange}
						/>
						<Checkbox
							label='Show ran tests only'
							checked={showOnlyRanTests}
							onChange={handleShowOnlyRanTestsChange}
						/>
						<Checkbox
							label='Show with annotations only:'
							checked={showOnlyTestsWithAnnotations}
							onChange={handleShowOnlyTestsWithAnnotationsChange}
						/>
						<Dropdown
							multiselect
							size='small'
							placeholder='Select annotations'
							defaultValue={selectedAnnotations.length ? selectedAnnotations.join(', ') : undefined}
							defaultSelectedOptions={selectedAnnotations}
							onOptionSelect={handleOptionSelect}
							onOpenChange={handleOpenChange}
						>
							{knownAnnotations.map((option) => (
								<Option key={option} text={option}>
									<Badge key={option} shape='square' appearance='outline' size='small'>{option}</Badge>
								</Option>
							))}
						</Dropdown>
					</div>
				</div>
			</div >
		);
	}
);
