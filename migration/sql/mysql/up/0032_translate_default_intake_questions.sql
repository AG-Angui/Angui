UPDATE intake_question_definitions
SET prompt = CASE id
    WHEN 'intake-q-0201' THEN '请填写可供家属核对的基本信息。'
    WHEN 'intake-q-0202' THEN '请补充健康、认知、行动能力或用药方面需要记录的情况。'
    WHEN 'intake-q-0203' THEN '请描述有助于后续核实线索的日常习惯、偏好或行为特点。'
    WHEN 'intake-q-0204' THEN '请说明最后出现的时间和地点；如有不确定的交通方式或同行人，也请标明。'
    WHEN 'intake-q-0205' THEN '请补充常去地点，并避免填写与寻找无关的私人住址。'
    WHEN 'intake-q-0206' THEN '是否有需要人工谨慎核实的可能原因、计划或担忧？不确定时可标记为未知。'
    WHEN 'intake-q-0207' THEN '请描述当时携带的衣着、包、手机、证件或其他随身物品。'
    WHEN 'intake-q-0208' THEN '请说明可能的独立出行方式，包括步行、车辆、公共交通及同行人情况。'
    WHEN 'intake-q-0209' THEN '是否有之后获得、但仍需要人工核实的信息或线索？'
END,
updated_at = '2026-08-01T00:00:00.000Z'
WHERE id IN ('intake-q-0201', 'intake-q-0202', 'intake-q-0203', 'intake-q-0204', 'intake-q-0205', 'intake-q-0206', 'intake-q-0207', 'intake-q-0208', 'intake-q-0209')
  AND version = 2
  AND (
    (id = 'intake-q-0201' AND prompt = 'Please describe the person using information your family can verify.') OR
    (id = 'intake-q-0202' AND prompt = 'What health, cognitive, mobility, or medication concerns should be recorded as unconfirmed draft information?') OR
    (id = 'intake-q-0203' AND prompt = 'What routines, preferences, or behaviors may help verify future leads?') OR
    (id = 'intake-q-0204' AND prompt = 'When and where was the person last seen? Include uncertainty in time, place, transport, or companions.') OR
    (id = 'intake-q-0205' AND prompt = 'Which places do they commonly visit? Please avoid unrelated private addresses.') OR
    (id = 'intake-q-0206' AND prompt = 'Are there any possible reasons, plans, or concerns that need careful human follow-up? Mark unknown when unsure.') OR
    (id = 'intake-q-0207' AND prompt = 'What clothing, bags, phone, identification, or other belongings were they carrying?') OR
    (id = 'intake-q-0208' AND prompt = 'How might they travel independently? Include walking, vehicle, public transport, and companion uncertainty.') OR
    (id = 'intake-q-0209' AND prompt = 'Is there later information or a lead that still needs human verification?')
  );
